import { clone, first, isEqual, last } from 'lodash';
import { Feature, LineString, Position } from 'geojson';
import { lineString, point } from '@turf/helpers';
import lineSlice from '@turf/line-slice';

import { Dispatch } from 'redux';
import { osrdEditoastApi, Identifier } from 'common/api/osrdEditoastApi';
import { getEntities, getEntity, getMixedEntities } from 'applications/editor/data/api';
import { DEFAULT_COMMON_TOOL_STATE } from 'applications/editor/tools/commonToolState';
import { RouteCandidate, RouteEditionState } from 'applications/editor/tools/routeEdition/types';
import {
  PartialButFor,
  RouteEntity,
  TrackRange,
  TrackSectionEntity,
  WayPoint,
  WayPointEntity,
} from 'types';

export function getEmptyCreateRouteState(): RouteEditionState {
  return {
    ...DEFAULT_COMMON_TOOL_STATE,
    type: 'editRoutePath',
    routeState: {
      entryPoint: null,
      entryPointDirection: 'START_TO_STOP',
      exitPoint: null,
    },
    optionsState: { type: 'idle' },
    extremityEditionState: { type: 'idle' },
  };
}

export function getEditRouteState(route: RouteEntity): RouteEditionState {
  return {
    ...DEFAULT_COMMON_TOOL_STATE,
    type: 'editRouteMetadata',
    routeEntity: clone(route),
    initialRouteEntity: clone(route),
  };
}

/**
 * This helper deletes consecutive repeted coordinates in an array of
 * coordinates:
 */
export function removeDuplicatePoints(coordinates: Position[]): Position[] {
  const res: Position[] = [];
  let lastPoint: Position | null = null;

  coordinates.forEach((coordinate) => {
    if (!isEqual(coordinate, lastPoint)) res.push(coordinate);
    lastPoint = coordinate;
  });

  return res;
}

export function computeRouteGeometry(
  tracks: Record<string, TrackSectionEntity>,
  entryPoint: WayPointEntity,
  exitPoint: WayPointEntity,
  trackRanges: PartialButFor<TrackRange, 'track' | 'direction'>[]
): Feature<LineString, { id: string }> {
  return lineString(
    removeDuplicatePoints(
      trackRanges.flatMap((range, i) => {
        const track = tracks[range.track];
        const { direction } = range;

        if (!track) throw new Error(`Track ${range.track} not found`);

        const isFirst = !i;
        const isLast = i === trackRanges.length - 1;

        const p1 = first(track.geometry.coordinates) as Position;
        const p2 = last(track.geometry.coordinates) as Position;

        // Weird case of only range:
        if (isFirst && isLast) {
          return direction === 'START_TO_STOP'
            ? lineSlice(entryPoint.geometry, exitPoint.geometry, track.geometry).geometry
                .coordinates
            : lineSlice(exitPoint.geometry, entryPoint.geometry, track.geometry)
                .geometry.coordinates.slice(0)
                .reverse();
        }

        // First:
        if (isFirst) {
          if (direction === 'START_TO_STOP') {
            return lineSlice(entryPoint.geometry, point(p2), track.geometry).geometry.coordinates;
          }

          return lineSlice(point(p1), entryPoint.geometry, track.geometry)
            .geometry.coordinates.slice(0)
            .reverse();
        }

        // Last (we don't know the direction, we must guess from previous point):
        if (isLast) {
          if (direction === 'START_TO_STOP') {
            return lineSlice(point(p1), exitPoint.geometry, track.geometry).geometry.coordinates;
          }

          return lineSlice(exitPoint.geometry, point(p2), track.geometry)
            .geometry.coordinates.slice(0)
            .reverse();
        }

        // All ranges inbetween:
        if (direction === 'START_TO_STOP') {
          return track.geometry.coordinates;
        }

        return track.geometry.coordinates.slice(0).reverse();
      })
    )
  );
}

export async function getRouteGeometry(
  infra: number,
  entryPoint: WayPointEntity,
  exitPoint: WayPointEntity,
  trackRanges: TrackRange[],
  dispatch: Dispatch
): Promise<Feature<LineString, { id: string }>> {
  if (!trackRanges.length) return lineString([]);

  const tracks = await getEntities<TrackSectionEntity>(
    infra,
    trackRanges.map((trackRange) => trackRange.track),
    'TrackSection',
    dispatch
  );

  return computeRouteGeometry(tracks, entryPoint, exitPoint, trackRanges);
}

async function getRouteGeometryByRoute(
  infra: number,
  route: RouteEntity,
  dispatch: Dispatch
): Promise<Feature<LineString, { id: string }>> {
  const trackRangesResult = await dispatch(
    osrdEditoastApi.endpoints.getInfraByInfraIdRoutesTrackRanges.initiate({
      infraId: infra as number,
      routes: route.properties.id,
    })
  ).unwrap();
  if (trackRangesResult.length === 0 || trackRangesResult[0].type !== 'Computed') {
    throw new Error('Some track ranges could not be computed yet.');
  }
  const trackRanges = trackRangesResult[0].track_ranges;

  const extremities = await getMixedEntities<WayPointEntity>(
    infra,
    [route.properties.entry_point, route.properties.exit_point],
    dispatch
  );
  const entryPoint = extremities[route.properties.entry_point.id];
  const exitPoint = extremities[route.properties.exit_point.id];

  if (!entryPoint)
    throw new Error(
      `Entry point ${route.properties.entry_point.id} (${route.properties.entry_point.type}) for route ${route.properties.id} not found`
    );
  if (!exitPoint)
    throw new Error(
      `Exit point ${route.properties.exit_point.id} (${route.properties.exit_point.type}) for route ${route.properties.id} not found`
    );

  return getRouteGeometry(infra, entryPoint, exitPoint, trackRanges, dispatch);
}

export async function getRouteGeometryByRouteId(
  infra: number,
  routeId: string,
  dispatch: Dispatch
): Promise<Feature<LineString, { id: string }>> {
  const route = await getEntity<RouteEntity>(infra, routeId, 'Route', dispatch);

  return getRouteGeometryByRoute(infra, route, dispatch);
}

export async function getRouteGeometries(
  infra: number,
  entryPoint: WayPoint,
  exitPoint: WayPoint,
  candidates: RouteCandidate[],
  dispatch: Dispatch
): Promise<Feature<LineString, { id: string }>[]> {
  const extremities = await getMixedEntities<WayPointEntity>(
    infra,
    [entryPoint, exitPoint],
    dispatch
  );
  const entryPointEntity = extremities[entryPoint.id];
  const exitPointEntity = extremities[exitPoint.id];

  if (!entryPointEntity)
    throw new Error(`Entry point ${entryPoint.id} (${entryPoint.type}) not found`);
  if (!exitPointEntity) throw new Error(`Exit point ${exitPoint.id} (${exitPoint.type}) not found`);

  return Promise.all(
    candidates.map((candidate) =>
      getRouteGeometry(
        infra,
        entryPointEntity,
        exitPointEntity,
        candidate.track_ranges as TrackRange[],
        dispatch
      )
    )
  );
}

export async function getCompatibleRoutesPayload(
  infra: number,
  entryPoint: WayPoint,
  exitPoint: WayPoint,
  dispatch: Dispatch
) {
  const extremities = await getMixedEntities<WayPointEntity>(
    infra,
    [entryPoint, exitPoint],
    dispatch
  );
  const entryPointEntity = extremities[entryPoint.id];
  const exitPointEntity = extremities[exitPoint.id];

  if (!entryPointEntity)
    throw new Error(`Entry point ${entryPoint.id} (${entryPoint.type}) not found`);
  if (!exitPointEntity) throw new Error(`Exit point ${exitPoint.id} (${exitPoint.type}) not found`);

  return {
    starting: {
      track: entryPointEntity.properties.track as Identifier,
      position: entryPointEntity.properties.position as number,
    },
    ending: {
      track: exitPointEntity.properties.track as Identifier,
      position: exitPointEntity.properties.position as number,
    },
  };
}
