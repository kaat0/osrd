use crate::core::pathfinding::InvalidPathItem;
use crate::core::pathfinding::PathfindingInputError;
use crate::error::Result;
use crate::models::TrackSectionModel;
use crate::RetrieveBatchUnchecked;
use editoast_schemas::infra::TrackOffset;
use editoast_schemas::primitives::NonBlankString;
use editoast_schemas::train_schedule::OperationalPointReference;
use editoast_schemas::train_schedule::TrackReference;
use std::collections::HashMap;
use std::collections::HashSet;

use editoast_models::DbConnection;
use editoast_schemas::train_schedule::OperationalPointIdentifier;
use editoast_schemas::train_schedule::PathItemLocation;

use crate::models::OperationalPointModel;

use super::pathfinding::PathfindingFailure;
use super::pathfinding::PathfindingResult;

type TrackOffsetResult = std::result::Result<Vec<Vec<TrackOffset>>, PathfindingResult>;

/// Gather information about several path items, factorizing db calls.
pub struct PathItemCache {
    uic_to_ops: HashMap<i64, Vec<OperationalPointModel>>,
    trigram_to_ops: HashMap<String, Vec<OperationalPointModel>>,
    ids_to_ops: HashMap<String, OperationalPointModel>,
    existing_track_ids: HashSet<String>,
    track_ids_to_name: HashMap<String, NonBlankString>,
}

impl PathItemCache {
    /// Load the path item cache from a list of pathfinding inputs
    pub async fn load(
        conn: &mut DbConnection,
        infra_id: i64,
        path_items: &[&PathItemLocation],
    ) -> Result<PathItemCache> {
        let (trigrams, ops_uic, ops_id) = collect_path_item_ids(path_items);
        let uic_to_ops = retrieve_op_from_uic(conn, infra_id, &ops_uic).await?;
        let trigram_to_ops = retrieve_op_from_trigrams(conn, infra_id, &trigrams).await?;
        let ids_to_ops = retrieve_op_from_ids(conn, infra_id, &ops_id).await?;

        let tracks = path_items.iter().filter_map(|item| match item {
            PathItemLocation::TrackOffset(TrackOffset { track, .. }) => {
                Some((infra_id, track.0.clone()))
            }
            _ => None,
        });
        let tracks = ids_to_ops
            .values()
            .chain(trigram_to_ops.values().flatten())
            .chain(uic_to_ops.values().flatten())
            .flat_map(|op| &op.parts)
            .map(|part| (infra_id, part.track.0.clone()))
            .chain(tracks);
        let track_sections =
            TrackSectionModel::retrieve_batch_unchecked::<_, Vec<_>>(conn, tracks.clone()).await?;
        let existing_track_ids = track_sections
            .clone()
            .into_iter()
            .map(|track| track.obj_id)
            .collect();
        let track_ids_to_name = track_sections
            .into_iter()
            .filter_map(|track| {
                track
                    .extensions
                    .sncf
                    .as_ref()
                    .map(|extension| (track.obj_id.clone(), extension.track_name.clone()))
            })
            .collect();

        Ok(PathItemCache {
            uic_to_ops,
            trigram_to_ops,
            ids_to_ops,
            existing_track_ids,
            track_ids_to_name,
        })
    }

    /// Get the operational points associated with an identifier
    pub fn get_from_id(&self, id: &str) -> Option<&OperationalPointModel> {
        self.ids_to_ops.get(id)
    }

    /// Get the operational points associated with a trigram
    pub fn get_from_trigram(&self, trigram: &str) -> Option<&Vec<OperationalPointModel>> {
        self.trigram_to_ops.get(trigram)
    }

    /// Get the operational points associated with a UIC code
    pub fn get_from_uic(&self, uic: i64) -> Option<&Vec<OperationalPointModel>> {
        self.uic_to_ops.get(&uic)
    }

    /// Check if a track exists
    pub fn track_exists(&self, track: &str) -> bool {
        self.existing_track_ids.contains(track)
    }

    /// Extract locations from path items
    pub fn extract_location_from_path_items(
        &self,
        path_items: &[&PathItemLocation],
    ) -> TrackOffsetResult {
        let mut result: Vec<Vec<_>> = Vec::default();
        let mut invalid_path_items = Vec::new();
        for (index, &path_item) in path_items.iter().enumerate() {
            let track_offsets = match path_item {
                PathItemLocation::TrackOffset(track_offset) => {
                    vec![track_offset.clone()]
                }
                PathItemLocation::OperationalPointReference(OperationalPointReference {
                    reference: OperationalPointIdentifier::OperationalPointId { operational_point },
                    track_reference,
                }) => {
                    let mut track_offsets = vec![];
                    if let Some(op) = self.get_from_id(&operational_point.0) {
                        track_offsets = op.track_offset();
                        track_offsets = self.track_reference_filter(track_offsets, track_reference);
                    }
                    if track_offsets.is_empty() {
                        invalid_path_items.push(InvalidPathItem {
                            index,
                            path_item: path_item.clone(),
                        });
                        continue;
                    }
                    track_offsets
                }
                PathItemLocation::OperationalPointReference(OperationalPointReference {
                    reference:
                        OperationalPointIdentifier::OperationalPointDescription {
                            trigram,
                            secondary_code,
                        },
                    track_reference,
                }) => {
                    let ops = self
                        .get_from_trigram(&trigram.0)
                        .cloned()
                        .unwrap_or_default();
                    let ops = secondary_code_filter(secondary_code, ops);
                    let track_offsets = track_offsets_from_ops(&ops);
                    let track_offsets = self.track_reference_filter(track_offsets, track_reference);
                    if track_offsets.is_empty() {
                        invalid_path_items.push(InvalidPathItem {
                            index,
                            path_item: path_item.clone(),
                        });
                        continue;
                    };
                    track_offsets
                }
                PathItemLocation::OperationalPointReference(OperationalPointReference {
                    reference:
                        OperationalPointIdentifier::OperationalPointUic {
                            uic,
                            secondary_code,
                        },
                    track_reference,
                }) => {
                    let ops = self
                        .get_from_uic(i64::from(*uic))
                        .cloned()
                        .unwrap_or_default();
                    let ops = secondary_code_filter(secondary_code, ops);
                    let track_offsets = track_offsets_from_ops(&ops);
                    let track_offsets = self.track_reference_filter(track_offsets, track_reference);
                    if track_offsets.is_empty() {
                        invalid_path_items.push(InvalidPathItem {
                            index,
                            path_item: path_item.clone(),
                        });
                        continue;
                    };
                    track_offsets
                }
            };

            // Check if tracks exist
            for track_offset in &track_offsets {
                if !self.track_exists(&track_offset.track.0) {
                    invalid_path_items.push(InvalidPathItem {
                        index,
                        path_item: path_item.clone(),
                    });
                    continue;
                }
            }

            result.push(track_offsets);
        }

        if !invalid_path_items.is_empty() {
            return Err(PathfindingResult::Failure(
                PathfindingFailure::PathfindingInputError(
                    PathfindingInputError::InvalidPathItems {
                        items: invalid_path_items,
                    },
                ),
            ));
        }

        Ok(result)
    }

    /// Filter operational points parts by a track label or a track id
    /// If neither a track label or id is provided, the original list is returned
    fn track_reference_filter(
        &self,
        track_offsets: Vec<TrackOffset>,
        track_reference: &Option<TrackReference>,
    ) -> Vec<TrackOffset> {
        match track_reference {
            Some(TrackReference::Id { track_id }) => track_offsets
                .into_iter()
                .filter(|track_offset| &track_offset.track == track_id)
                .collect(),
            Some(TrackReference::Name { track_name }) => track_offsets
                .into_iter()
                .filter(|track_offset| {
                    self.track_ids_to_name
                        .get::<String>(&track_offset.track)
                        .unwrap()
                        == track_name
                })
                .collect(),
            None => track_offsets,
        }
    }
}

/// Collect the ids of the operational points from the path items
fn collect_path_item_ids(path_items: &[&PathItemLocation]) -> (Vec<String>, Vec<i64>, Vec<String>) {
    let mut trigrams: Vec<String> = Vec::new();
    let mut ops_uic: Vec<i64> = Vec::new();
    let mut ops_id: Vec<String> = Vec::new();

    for item in path_items {
        match item {
            PathItemLocation::OperationalPointReference(OperationalPointReference {
                reference: OperationalPointIdentifier::OperationalPointDescription { trigram, .. },
                ..
            }) => {
                trigrams.push(trigram.clone().0);
            }
            PathItemLocation::OperationalPointReference(OperationalPointReference {
                reference: OperationalPointIdentifier::OperationalPointUic { uic, .. },
                ..
            }) => {
                ops_uic.push(i64::from(*uic));
            }
            PathItemLocation::OperationalPointReference(OperationalPointReference {
                reference:
                    OperationalPointIdentifier::OperationalPointId {
                        operational_point, ..
                    },
                ..
            }) => {
                ops_id.push(operational_point.clone().0);
            }
            _ => {}
        }
    }
    (trigrams, ops_uic, ops_id)
}

/// Retrieve operational points from operational point uic codes
async fn retrieve_op_from_uic(
    conn: &mut DbConnection,
    infra_id: i64,
    ops_uic: &[i64],
) -> Result<HashMap<i64, Vec<OperationalPointModel>>> {
    let mut uic_to_ops: HashMap<i64, Vec<OperationalPointModel>> = HashMap::new();
    OperationalPointModel::retrieve_from_uic(conn, infra_id, ops_uic)
        .await?
        .into_iter()
        .for_each(|op| {
            uic_to_ops
                .entry(op.extensions.identifier.clone().unwrap().uic)
                .or_default()
                .push(op)
        });
    Ok(uic_to_ops)
}

/// Retrieve operational points from operational point trigams
async fn retrieve_op_from_trigrams(
    conn: &mut DbConnection,
    infra_id: i64,
    trigrams: &[String],
) -> Result<HashMap<String, Vec<OperationalPointModel>>> {
    let mut trigrams_to_ops: HashMap<String, Vec<OperationalPointModel>> = HashMap::new();
    OperationalPointModel::retrieve_from_trigrams(conn, infra_id, trigrams)
        .await?
        .into_iter()
        .for_each(|op| {
            trigrams_to_ops
                .entry(op.extensions.sncf.clone().unwrap().trigram)
                .or_default()
                .push(op)
        });
    Ok(trigrams_to_ops)
}

/// Retrieve operational points from operational point ids
async fn retrieve_op_from_ids(
    conn: &mut DbConnection,
    infra_id: i64,
    ops_id: &[String],
) -> Result<HashMap<String, OperationalPointModel>> {
    let ops_id = ops_id.iter().map(|obj_id| (infra_id, obj_id.clone()));
    // a check for missing ids is performed later
    let ids_to_ops: HashMap<_, _> =
        OperationalPointModel::retrieve_batch_unchecked::<_, Vec<_>>(conn, ops_id)
            .await?
            .into_iter()
            .map(|op| (op.obj_id.clone(), op))
            .collect();

    Ok(ids_to_ops)
}

fn track_offsets_from_ops(ops: &[OperationalPointModel]) -> Vec<TrackOffset> {
    ops.iter().flat_map(|op| op.track_offset()).collect()
}

/// Filter operational points by secondary code
/// If the secondary code is not provided, the original list is returned
fn secondary_code_filter(
    secondary_code: &Option<String>,
    ops: Vec<OperationalPointModel>,
) -> Vec<OperationalPointModel> {
    if let Some(secondary_code) = secondary_code {
        ops.into_iter()
            .filter(|op| &op.extensions.sncf.as_ref().unwrap().ch == secondary_code)
            .collect()
    } else {
        ops
    }
}
