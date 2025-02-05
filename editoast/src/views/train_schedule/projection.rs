use axum::extract::Json;
use axum::extract::State;
use axum::Extension;
use chrono::DateTime;
use chrono::Utc;
use editoast_authz::BuiltinRole;
use editoast_schemas::primitives::Identifier;
use futures::join;
use itertools::izip;
use serde::Deserialize;
use serde::Serialize;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::collections::HashSet;
use std::hash::Hash;
use std::hash::Hasher;
use std::sync::Arc;
use tracing::info;
use utoipa::ToSchema;

use super::TrainScheduleError;
use crate::client::get_app_version;
use crate::core::pathfinding::PathfindingResultSuccess;
use crate::core::pathfinding::TrackRange;
use crate::core::signal_projection::SignalUpdate;
use crate::core::signal_projection::SignalUpdatesRequest;
use crate::core::signal_projection::TrainSimulation;
use crate::core::simulation::SimulationResponse;
use crate::core::AsCoreRequest;
use crate::core::CoreClient;
use crate::error::Result;
use crate::models::infra::Infra;
use crate::models::train_schedule::TrainSchedule;
use crate::models::Retrieve;
use crate::models::RetrieveBatch;
use crate::views::path::pathfinding::PathfindingResult;
use crate::views::path::projection::PathProjection;
use crate::views::path::projection::TrackLocationFromPath;
use crate::views::train_schedule::train_simulation_batch;
use crate::views::train_schedule::CompleteReportTrain;
use crate::views::train_schedule::ReportTrain;
use crate::views::train_schedule::SignalCriticalPosition;
use crate::views::train_schedule::ZoneUpdate;
use crate::views::AuthenticationExt;
use crate::views::AuthorizationError;
use crate::AppState;
use crate::RollingStockModel;

editoast_common::schemas! {
    ProjectPathTrainResult,
    ProjectPathForm,
}
crate::routes! {
    "/project_path" => project_path,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ProjectPathForm {
    infra_id: i64,
    electrical_profile_set_id: Option<i64>,
    ids: HashSet<i64>,
    #[schema(inline)]
    path: ProjectPathInput,
}

/// Project path input is described by a list of routes and a list of track range
#[derive(Debug, Deserialize, ToSchema)]
struct ProjectPathInput {
    /// List of track ranges
    #[schema(min_items = 1)]
    track_section_ranges: Vec<TrackRange>,
    /// List of route ids
    #[schema(inline, min_items = 1)]
    routes: Vec<Identifier>,
    /// Path description as block ids
    #[schema(inline, min_items = 1)]
    blocks: Vec<Identifier>,
}

#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
struct SpaceTimeCurve {
    // List of positions of a train in mm
    // Both positions and times must have the same length
    #[schema(min_items = 2)]
    positions: Vec<u64>,
    // List of times in ms since `departure_time` associated to a position
    #[schema(min_items = 2)]
    times: Vec<u64>,
}

/// Project path output is described by time-space points and blocks
#[derive(Debug, Deserialize, Serialize, ToSchema)]
struct ProjectPathTrainResult {
    /// Departure time of the train
    departure_time: DateTime<Utc>,
    /// Rolling stock length in mm
    rolling_stock_length: u64,
    #[serde(flatten)]
    #[schema(inline)]
    cached: CachedProjectPathTrainResult,
}

/// Project path output is described by time-space points and blocks
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
struct CachedProjectPathTrainResult {
    /// List of space-time curves sections along the path
    #[schema(inline)]
    space_time_curves: Vec<SpaceTimeCurve>,
    /// List of signal updates along the path
    #[schema(inline)]
    signal_updates: Vec<SignalUpdate>,
}

/// Projects the space time curves and paths of a number of train schedules onto a given path
///
/// - Returns 404 if the infra or any of the train schedules are not found
/// - Returns 200 with a hashmap of train_id to ProjectPathTrainResult
///
/// Train schedules that are invalid (pathfinding or simulation failed) are not included in the result
#[utoipa::path(
    post, path = "",
    tag = "train_schedule",
    request_body = ProjectPathForm,
    responses(
        (status = 200, description = "Project Path Output", body = HashMap<i64, ProjectPathTrainResult>),
    ),
)]
async fn project_path(
    State(AppState {
        db_pool,
        valkey: valkey_client,
        core_client,
        ..
    }): State<AppState>,
    Extension(auth): AuthenticationExt,
    Json(ProjectPathForm {
        infra_id,
        ids: train_ids,
        path,
        electrical_profile_set_id,
    }): Json<ProjectPathForm>,
) -> Result<Json<HashMap<i64, ProjectPathTrainResult>>> {
    let authorized = auth
        .check_roles(
            [
                BuiltinRole::InfraRead,
                BuiltinRole::TimetableRead,
                BuiltinRole::RollingStockCollectionRead,
            ]
            .into(),
        )
        .await
        .map_err(AuthorizationError::AuthError)?;
    if !authorized {
        return Err(AuthorizationError::Forbidden.into());
    }

    let ProjectPathInput {
        track_section_ranges: path_track_ranges,
        routes: path_routes,
        blocks: path_blocks,
    } = path;
    let path_projection = PathProjection::new(&path_track_ranges);
    let mut valkey_conn = valkey_client.get_connection().await?;

    let infra = Infra::retrieve_or_fail(&mut db_pool.get().await?, infra_id, || {
        TrainScheduleError::InfraNotFound { infra_id }
    })
    .await?;

    let trains: Vec<TrainSchedule> =
        TrainSchedule::retrieve_batch_or_fail(&mut db_pool.get().await?, train_ids, |missing| {
            TrainScheduleError::BatchTrainScheduleNotFound {
                number: missing.len(),
            }
        })
        .await?;

    let (rolling_stocks, _): (Vec<_>, _) = RollingStockModel::retrieve_batch(
        &mut db_pool.get().await?,
        trains
            .iter()
            .map::<String, _>(|t| t.rolling_stock_name.clone()),
    )
    .await?;

    let simulations = train_simulation_batch(
        &mut db_pool.get().await?,
        valkey_client.clone(),
        core_client.clone(),
        &trains,
        &infra,
        electrical_profile_set_id,
    )
    .await?;

    // 1. Retrieve cached projection
    let mut trains_hash_values = vec![];
    let mut trains_details = vec![];

    for (train, (sim, pathfinding_result)) in izip!(&trains, simulations) {
        let track_ranges = match pathfinding_result {
            PathfindingResult::Success(PathfindingResultSuccess {
                track_section_ranges,
                ..
            }) => track_section_ranges,
            _ => continue,
        };

        let CompleteReportTrain {
            report_train,
            signal_critical_positions,
            zone_updates,
            ..
        } = match sim {
            SimulationResponse::Success { final_output, .. } => final_output,
            _ => continue,
        };
        let ReportTrain {
            times, positions, ..
        } = report_train;

        let train_details = TrainSimulationDetails {
            train_id: train.id,
            positions,
            times,
            signal_critical_positions,
            zone_updates,
            train_path: track_ranges,
        };

        let hash = train_projection_input_hash(
            infra.id,
            &infra.version,
            &train_details,
            &path_track_ranges,
            &path_routes,
            &path_blocks,
        );
        trains_hash_values.push(hash);
        trains_details.push(train_details);
    }
    let cached_projections: Vec<Option<CachedProjectPathTrainResult>> =
        valkey_conn.json_get_bulk(&trains_hash_values).await?;

    let mut hit_cache = vec![];
    let mut miss_cache = vec![];
    for (train_details, projection) in izip!(&trains_details, cached_projections) {
        if let Some(cached) = projection {
            hit_cache.push((cached, train_details.train_id));
        } else {
            miss_cache.push(train_details.clone());
        }
    }

    info!(
        nb_hit = hit_cache.len(),
        nb_miss = miss_cache.len(),
        "Hit cache",
    );

    // 2 Compute space time curves and signal updates for all miss cache
    let (space_time_curves, signal_updates) = join!(
        compute_batch_space_time_curves(&miss_cache, &path_projection),
        compute_batch_signal_updates(
            core_client.clone(),
            &infra,
            &path_track_ranges,
            &path_routes,
            &path_blocks,
            &miss_cache
        )
    );
    let signal_updates = signal_updates?;

    // 3. Store the projection in the cache (using pipeline)
    let trains_hash_values: HashMap<_, _> = trains_details
        .iter()
        .map(|t| t.train_id)
        .zip(trains_hash_values)
        .collect();
    let mut new_items = vec![];
    for train_id in miss_cache.iter().map(|t| t.train_id) {
        let hash = &trains_hash_values[&train_id];
        let cached_value = CachedProjectPathTrainResult {
            space_time_curves: space_time_curves
                .get(&train_id)
                .expect("Space time curves not available for train")
                .clone(),
            signal_updates: signal_updates
                .get(&train_id)
                .expect("Signal update not available for train")
                .clone(),
        };
        hit_cache.push((cached_value.clone(), train_id));
        new_items.push((hash, cached_value));
    }
    valkey_conn.json_set_bulk(&new_items).await?;

    let train_map: HashMap<i64, TrainSchedule> = trains.into_iter().map(|ts| (ts.id, ts)).collect();

    // 4.1 Fetch rolling stock length
    let rolling_stock_length: HashMap<_, _> = rolling_stocks
        .into_iter()
        .map(|rs| (rs.name, rs.length))
        .collect();

    // 4.2 Build the projection response
    let mut project_path_result = HashMap::new();
    for (cached, train_id) in hit_cache {
        let train = train_map.get(&train_id).expect("Train not found");
        let length = rolling_stock_length
            .get(&train.rolling_stock_name)
            .expect("Rolling stock length not found");

        project_path_result.insert(
            train_id,
            ProjectPathTrainResult {
                departure_time: train.start_time,
                rolling_stock_length: (length * 1000.).round() as u64,
                cached,
            },
        );
    }

    Ok(Json(project_path_result))
}

/// Input for the projection of a train schedule on a path
#[derive(Debug, Clone, Hash)]
struct TrainSimulationDetails {
    train_id: i64,
    positions: Vec<u64>,
    times: Vec<u64>,
    train_path: Vec<TrackRange>,
    signal_critical_positions: Vec<SignalCriticalPosition>,
    zone_updates: Vec<ZoneUpdate>,
}

/// Compute the signal updates of a list of train schedules
async fn compute_batch_signal_updates<'a>(
    core: Arc<CoreClient>,
    infra: &Infra,
    path_track_ranges: &'a Vec<TrackRange>,
    path_routes: &'a Vec<Identifier>,
    path_blocks: &'a Vec<Identifier>,
    trains_details: &'a [TrainSimulationDetails],
) -> Result<HashMap<i64, Vec<SignalUpdate>>> {
    if trains_details.is_empty() {
        return Ok(HashMap::new());
    }
    let request = SignalUpdatesRequest {
        infra: infra.id,
        expected_version: infra.version.clone(),
        track_section_ranges: path_track_ranges,
        routes: path_routes,
        blocks: path_blocks,
        train_simulations: trains_details
            .iter()
            .map(|detail| {
                (
                    detail.train_id,
                    TrainSimulation {
                        signal_critical_positions: &detail.signal_critical_positions,
                        zone_updates: &detail.zone_updates,
                        simulation_end_time: detail.times[detail.times.len() - 1],
                    },
                )
            })
            .collect(),
    };
    let response = request.fetch(&core).await?;
    Ok(response.signal_updates)
}

/// Compute space time curves of a list of train schedules
async fn compute_batch_space_time_curves(
    trains_details: &Vec<TrainSimulationDetails>,
    path_projection: &PathProjection<'_>,
) -> HashMap<i64, Vec<SpaceTimeCurve>> {
    let mut space_time_curves = HashMap::new();

    for train_detail in trains_details {
        space_time_curves.insert(
            train_detail.train_id,
            compute_space_time_curves(train_detail, path_projection),
        );
    }
    space_time_curves
}

/// Compute the space time curves of a train schedule on a path
fn compute_space_time_curves(
    project_path_input: &TrainSimulationDetails,
    path_projection: &PathProjection,
) -> Vec<SpaceTimeCurve> {
    let train_path = PathProjection::new(&project_path_input.train_path);
    let intersections = path_projection.get_intersections(&project_path_input.train_path);
    let positions = &project_path_input.positions;
    let times = &project_path_input.times;

    assert_eq!(positions[0], 0);
    assert_eq!(positions[positions.len() - 1], train_path.len());
    assert_eq!(positions.len(), times.len());

    let mut space_time_curves = vec![];
    for intersection in intersections {
        let start = intersection.start();
        let end = intersection.end();
        let start_index = find_index_upper(positions, start);
        let end_index = find_index_upper(positions, end);

        // Each segment contains the start, end and all positions between them
        // We must interpolate the start and end positions if they are not part of the positions
        let mut segment_positions = Vec::with_capacity(end_index - start_index + 2);
        let mut segment_times = Vec::with_capacity(end_index - start_index + 2);
        if positions[start_index] > start {
            // Interpolate the first point of the segment
            segment_positions.push(project_pos(start, &train_path, path_projection));
            segment_times.push(interpolate(
                positions[start_index - 1],
                positions[start_index],
                times[start_index - 1],
                times[start_index],
                start,
            ));
        }

        // Project all the points in the segment
        for index in start_index..end_index {
            segment_positions.push(project_pos(positions[index], &train_path, path_projection));
            segment_times.push(times[index]);
        }

        // Interpolate the last point of the segment
        segment_positions.push(project_pos(end, &train_path, path_projection));
        segment_times.push(interpolate(
            positions[end_index - 1],
            positions[end_index],
            times[end_index - 1],
            times[end_index],
            end,
        ));
        space_time_curves.push(SpaceTimeCurve {
            positions: segment_positions,
            times: segment_times,
        });
    }
    space_time_curves
}

/// Find the index of the first element greater to a value
///
/// **Values must be sorted in ascending order**
///
/// ## Panics
///
/// - If value is greater than the last element of values.
/// - If values is empty
fn find_index_upper(values: &[u64], value: u64) -> usize {
    assert!(!values.is_empty(), "Values can't be empty");
    assert!(
        value <= values[values.len() - 1],
        "Value can't be greater than the last element"
    );
    // Binary search that retrieve the smallest index of the first element greater than value
    let mut left = 0;
    let mut right = values.len();
    while left < right {
        let mid = (left + right) / 2;
        if values[mid] > value {
            right = mid;
        } else {
            left = mid + 1;
        }
    }
    if values[right - 1] == value {
        right - 1
    } else {
        right
    }
}

/// Project a position on a train path to a position on a projection path
///
/// ## Panics
///
/// Panics if the position is not part of **both** paths
fn project_pos(
    train_pos: u64,
    train_path: &PathProjection,
    path_projection: &PathProjection,
) -> u64 {
    match train_path.get_location(train_pos) {
        TrackLocationFromPath::One(loc) => path_projection
            .get_position(&loc)
            .expect("Position should be in the projection path"),
        TrackLocationFromPath::Two(loc_a, loc_b) => {
            path_projection.get_position(&loc_a).unwrap_or_else(|| {
                path_projection
                    .get_position(&loc_b)
                    .expect("Position should be in the projection path")
            })
        }
    }
}

/// Interpolate a time value between two positions
fn interpolate(
    start_pos: u64,
    end_pos: u64,
    start_time: u64,
    end_time: u64,
    pos_to_interpolate: u64,
) -> u64 {
    if start_pos == end_pos {
        start_time
    } else {
        start_time
            + (pos_to_interpolate - start_pos) * (end_time - start_time) / (end_pos - start_pos)
    }
}

// Compute hash input of the projection of a train schedule on a path
fn train_projection_input_hash(
    infra_id: i64,
    infra_version: &String,
    project_path_input: &TrainSimulationDetails,
    path_projection_tracks: &[TrackRange],
    path_routes: &[Identifier],
    path_blocks: &[Identifier],
) -> String {
    let osrd_version = get_app_version().unwrap_or_default();
    let mut hasher = DefaultHasher::new();
    project_path_input.hash(&mut hasher);
    path_projection_tracks.hash(&mut hasher);
    path_routes.hash(&mut hasher);
    path_blocks.hash(&mut hasher);
    let hash_simulation_input = hasher.finish();
    format!("projection_{osrd_version}.{infra_id}.{infra_version}.{hash_simulation_input}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use editoast_schemas::infra::Direction;
    use rstest::rstest;

    #[rstest]
    #[case(1, 0)]
    #[case(2, 1)]
    #[case(3, 1)]
    #[case(4, 2)]
    #[case(5, 3)]
    #[case(6, 4)]
    #[case(7, 4)]
    #[case(8, 5)]
    #[case(9, 6)]
    fn test_find_index_upper(#[case] value: u64, #[case] expected: usize) {
        let values = vec![1, 3, 4, 5, 7, 8, 9];
        assert_eq!(find_index_upper(&values, value), expected);
    }

    #[rstest]
    fn test_compute_space_time_curves_case_1() {
        let positions: Vec<u64> = vec![0, 100, 200, 300, 400, 600, 730, 1000];
        let times: Vec<u64> = vec![0, 10, 20, 30, 40, 50, 70, 90];
        let path = vec![
            TrackRange::new("A", 0, 100, Direction::StartToStop),
            TrackRange::new("B", 0, 200, Direction::StopToStart),
            TrackRange::new("C", 0, 300, Direction::StartToStop),
            TrackRange::new("D", 120, 250, Direction::StopToStart),
        ];
        let path_projection = PathProjection::new(&path);

        let train_path = vec![
            TrackRange::new("A", 0, 100, Direction::StartToStop),
            TrackRange::new("B", 0, 200, Direction::StopToStart),
            TrackRange::new("C", 0, 300, Direction::StartToStop),
            TrackRange::new("D", 0, 250, Direction::StopToStart),
            TrackRange::new("E", 0, 150, Direction::StartToStop),
        ];

        let project_path_input = TrainSimulationDetails {
            train_id: 0,
            positions,
            times,
            train_path,
            signal_critical_positions: vec![],
            zone_updates: vec![],
        };

        let space_time_curves = compute_space_time_curves(&project_path_input, &path_projection);
        assert_eq!(space_time_curves.clone().len(), 1);
        let curve = &space_time_curves[0];
        assert_eq!(curve.times.len(), curve.positions.len());
        assert_eq!(curve.positions, vec![0, 100, 200, 300, 400, 600, 730]);
    }

    #[rstest]
    fn test_compute_space_time_curves_case_2() {
        let positions: Vec<u64> = vec![0, 100, 200, 300, 400, 730];
        let times: Vec<u64> = vec![0, 10, 20, 30, 40, 70];
        let path = vec![
            TrackRange::new("A", 0, 100, Direction::StartToStop),
            TrackRange::new("B", 0, 200, Direction::StopToStart),
            TrackRange::new("C", 0, 300, Direction::StartToStop),
            TrackRange::new("D", 120, 250, Direction::StopToStart),
        ];
        let path_projection = PathProjection::new(&path);

        let train_path = vec![
            TrackRange::new("A", 0, 100, Direction::StartToStop),
            TrackRange::new("B", 0, 200, Direction::StopToStart),
            TrackRange::new("C", 0, 300, Direction::StartToStop),
            TrackRange::new("D", 120, 250, Direction::StopToStart),
        ];

        let project_path_input = TrainSimulationDetails {
            train_id: 0,
            positions: positions.clone(),
            times: times.clone(),
            train_path,
            signal_critical_positions: vec![],
            zone_updates: vec![],
        };

        let space_time_curves = compute_space_time_curves(&project_path_input, &path_projection);
        assert_eq!(space_time_curves.clone().len(), 1);
        let curve = &space_time_curves[0];
        assert_eq!(curve.positions, positions);
        assert_eq!(curve.times, times);
    }

    #[rstest]
    fn test_compute_space_time_curves_case_3() {
        let positions: Vec<u64> = vec![0, 100, 200, 300, 400, 450, 500, 600, 720];
        let times: Vec<u64> = vec![0, 10, 20, 30, 40, 50, 60, 70, 80];
        let train_path = vec![
            TrackRange::new("A", 50, 100, Direction::StartToStop),
            TrackRange::new("B", 0, 200, Direction::StartToStop),
            TrackRange::new("X", 0, 100, Direction::StartToStop),
            TrackRange::new("C", 0, 200, Direction::StopToStart),
            TrackRange::new("Z", 0, 100, Direction::StartToStop),
            TrackRange::new("E", 30, 100, Direction::StartToStop),
        ];

        let path = vec![
            TrackRange::new("A", 0, 100, Direction::StartToStop),
            TrackRange::new("B", 0, 200, Direction::StartToStop),
            TrackRange::new("C", 0, 300, Direction::StartToStop),
            TrackRange::new("D", 0, 250, Direction::StopToStart),
            TrackRange::new("E", 25, 100, Direction::StopToStart),
        ];
        let path_projection = PathProjection::new(&path);

        let project_path_input = TrainSimulationDetails {
            train_id: 0,
            positions,
            times,
            train_path,
            signal_critical_positions: vec![],
            zone_updates: vec![],
        };

        let space_time_curves = compute_space_time_curves(&project_path_input, &path_projection);
        assert_eq!(space_time_curves.clone().len(), 3);
        let curve = &space_time_curves[0];
        assert_eq!(curve.positions, vec![50, 150, 250, 300]);
        assert_eq!(curve.times, vec![0, 10, 20, 25]);

        let curve = &space_time_curves[1];
        assert_eq!(curve.positions, vec![500, 450, 400, 350, 300]);
        assert_eq!(curve.times, vec![35, 40, 50, 60, 65]);

        let curve = &space_time_curves[2];
        assert_eq!(curve.positions, vec![920, 850]);
        assert_eq!(curve.times, vec![74, 80]);
    }
}
