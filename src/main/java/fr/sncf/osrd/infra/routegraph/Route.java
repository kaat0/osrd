package fr.sncf.osrd.infra.routegraph;

import edu.umd.cs.findbugs.annotations.SuppressFBWarnings;
import fr.sncf.osrd.infra.TVDSection;
import fr.sncf.osrd.infra.railscript.value.RSMatchable;
import fr.sncf.osrd.infra.railscript.value.RSValue;
import fr.sncf.osrd.infra.signaling.Signal;
import fr.sncf.osrd.infra.trackgraph.Switch;
import fr.sncf.osrd.infra.trackgraph.SwitchPosition;
import fr.sncf.osrd.infra.waypointgraph.TVDSectionPath;
import fr.sncf.osrd.simulation.*;
import fr.sncf.osrd.train.TrackSectionRange;
import fr.sncf.osrd.utils.SortedArraySet;
import fr.sncf.osrd.utils.TrackSectionLocation;
import fr.sncf.osrd.utils.graph.BiNEdge;
import fr.sncf.osrd.utils.graph.EdgeDirection;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;

public class Route extends BiNEdge<Route> {
    public final String id;
    /** List of tvdSectionPath forming the route */
    public final List<TVDSectionPath> tvdSectionsPaths;
    @SuppressFBWarnings({"URF_UNREAD_PUBLIC_OR_PROTECTED_FIELD"})
    public final List<EdgeDirection> tvdSectionsPathDirections;
    public final List<SortedArraySet<TVDSection>> releaseGroups;
    public final HashMap<Switch, SwitchPosition> switchesPosition;
    public ArrayList<Signal> signalSubscribers;

    protected Route(
            String id,
            RouteGraph graph,
            double length,
            List<SortedArraySet<TVDSection>> releaseGroups,
            List<TVDSectionPath> tvdSectionsPaths,
            List<EdgeDirection> tvdSectionsPathDirections,
            HashMap<Switch, SwitchPosition> switchesPosition
    ) {
        super(
                graph.nextEdgeIndex(),
                tvdSectionsPaths.get(0).startNode,
                tvdSectionsPaths.get(tvdSectionsPaths.size() - 1).startNode,
                length
        );
        this.id = id;
        this.releaseGroups = releaseGroups;
        this.tvdSectionsPathDirections = tvdSectionsPathDirections;
        this.switchesPosition = switchesPosition;
        graph.registerEdge(this);
        this.tvdSectionsPaths = tvdSectionsPaths;
        this.signalSubscribers = new ArrayList<>();
    }

    /** Build track section path. Need to concatenate all track section of all TvdSectionPath.
     * Avoid to have in the path TrackSectionPositions that reference the same TrackSection. */
    public static ArrayList<TrackSectionRange> routesToTrackSectionRange(
            List<Route> routePath,
            TrackSectionLocation beginLocation,
            TrackSectionLocation endLocation
    ) {
        // Flatten the list of track section range
        var flattenSections = new ArrayDeque<TrackSectionRange>();
        for (var route : routePath) {
            for (var i = 0; i < route.tvdSectionsPaths.size(); i++) {
                var tvdSectionPath = route.tvdSectionsPaths.get(i);
                var tvdSectionPathDir = route.tvdSectionsPathDirections.get(i);
                for (var trackSection : tvdSectionPath.getTrackSections(tvdSectionPathDir))
                    flattenSections.addLast(trackSection);
            }
        }

        // Drop first track sections until the begin location
        while (true) {
            if (flattenSections.isEmpty())
                throw new RuntimeException("Begin position not contained in the route path");
            var firstTrack = flattenSections.removeFirst();
            if (firstTrack.containsLocation(beginLocation)) {
                var newTrackSection = new TrackSectionRange(firstTrack.edge, firstTrack.direction,
                        beginLocation.offset, firstTrack.getEndPosition());
                flattenSections.addFirst(newTrackSection);
                break;
            }
        }

        // Drop lasts track sections until the end location
        while (true) {
            if (flattenSections.isEmpty())
                throw new RuntimeException("End position not contained in the route path");
            var lastTrack = flattenSections.removeLast();
            if (lastTrack.containsLocation(endLocation)) {
                var newTrackSection = new TrackSectionRange(lastTrack.edge, lastTrack.direction,
                        lastTrack.getBeginPosition(), endLocation.offset);
                flattenSections.addLast(newTrackSection);
                break;
            }
        }

        // Merge duplicated edges
        var trackSectionPath = new ArrayList<TrackSectionRange>();
        TrackSectionRange lastTrack = flattenSections.removeFirst();
        while (!flattenSections.isEmpty()) {
            var currentTrack = flattenSections.removeFirst();
            if (lastTrack.edge != currentTrack.edge) {
                trackSectionPath.add(lastTrack);
                lastTrack = currentTrack;
                continue;
            }
            lastTrack = TrackSectionRange.merge(lastTrack, currentTrack);
        }
        trackSectionPath.add(lastTrack);
        return trackSectionPath;
    }

    public State newState() {
        return new State(this);
    }

    /** The state of the route is the actual entity which interacts with the rest of the infrastructure */
    @SuppressFBWarnings({"URF_UNREAD_PUBLIC_OR_PROTECTED_FIELD"})
    public static final class State implements RSMatchable {
        public final Route route;
        public RouteStatus status;

        State(Route route) {
            this.route = route;
            this.status = RouteStatus.FREE;
        }

        /** Notify the route that one of his tvd section isn't occupied anymore */
        public void onTvdSectionUnoccupied(Simulation sim, TVDSection.State tvdSectionUnoccupied) {
            if (status != RouteStatus.OCCUPIED)
                return;

            // TODO This function could be optimized.
            // One way to do it is to add an attribute to tvdSection to know if they're occupied
            var tvdSectionsBehind = new SortedArraySet<TVDSection>();
            for (var tvdSectionPath : route.tvdSectionsPaths) {
                tvdSectionsBehind.add(tvdSectionPath.tvdSection);
                if (tvdSectionPath.tvdSection == tvdSectionUnoccupied.tvdSection)
                    break;
            }

            for (var releaseGroup : route.releaseGroups) {
                if (!releaseGroup.contains(tvdSectionUnoccupied.tvdSection))
                    continue;
                if (!tvdSectionsBehind.contains(releaseGroup))
                    continue;
                for (var releasedTvdSection : releaseGroup) {
                    var tvdSection = sim.infraState.getTvdSectionState(releasedTvdSection.index);
                    if (tvdSection.isReserved())
                        tvdSection.free(sim);
                }
            }
        }

        /** Notify the route that one of his tvd section was freed */
        public void onTvdSectionFreed(Simulation sim) {
            if (status == RouteStatus.FREE)
                return;

            // Check that all tvd sections are free to free the route
            for (var tvdSectionPath : route.tvdSectionsPaths) {
                var tvdSection = sim.infraState.getTvdSectionState(tvdSectionPath.tvdSection.index);
                if (tvdSection.isReserved())
                    return;
            }

            var change = new RouteStatusChange(sim, this, RouteStatus.FREE);
            change.apply(sim, this);
            sim.publishChange(change);
            notifySignals(sim);
        }

        /** Notify the route that one of his tvd section is reserved */
        public void onTvdSectionReserved(Simulation sim) {
            if (status != RouteStatus.FREE)
                return;
            var change = new Route.RouteStatusChange(sim, this, RouteStatus.CONFLICT);
            change.apply(sim, this);
            sim.publishChange(change);
            notifySignals(sim);
        }

        /** Notify the route that one of his tvd section is occupied */
        public void onTvdSectionOccupied(Simulation sim) {
            if (status != RouteStatus.RESERVED)
                return;
            var change = new RouteStatusChange(sim, this, RouteStatus.OCCUPIED);
            change.apply(sim, this);
            sim.publishChange(change);
            notifySignals(sim);
        }

        private void notifySignals(Simulation sim) {
            for (var signal : route.signalSubscribers) {
                var signalState = sim.infraState.getSignalState(signal.index);
                signalState.notifyChange(sim);
            }
        }

        /** Reserve a route and his tvd sections. Routes that share tvd sections will have the status CONFLICT */
        public void reserve(Simulation sim) {
            assert status == RouteStatus.FREE;
            var change = new RouteStatusChange(sim, this, RouteStatus.RESERVED);
            change.apply(sim, this);
            sim.publishChange(change);
            notifySignals(sim);

            // Reserve the tvd sections
            for (var tvdSectionPath : route.tvdSectionsPaths) {
                var tvdSection = sim.infraState.getTvdSectionState(tvdSectionPath.tvdSection.index);
                tvdSection.reserve(sim);
            }

            // Set the switches in the required position
            for (var switchPos : route.switchesPosition.entrySet()) {
                var switchState = sim.infraState.getSwitchState(switchPos.getKey().switchIndex);
                switchState.setPosition(sim, switchPos.getValue());
            }
        }

        @Override
        public int getEnumValue() {
            return status.ordinal();
        }

        @Override
        @SuppressFBWarnings({"BC_UNCONFIRMED_CAST"})
        public boolean deepEquals(RSValue other) {
            if (other.getClass() != Route.State.class)
                return false;
            var o = (Route.State) other;
            if (route != o.route)
                return false;
            return status == o.status;
        }
    }

    public static class RouteStatusChange extends EntityChange<Route.State, Void> {
        public final RouteStatus newStatus;
        public final int routeIndex;

        /** create a RouteStatusChange */
        public RouteStatusChange(Simulation sim, Route.State entity, RouteStatus newStatus) {
            super(sim);
            this.newStatus = newStatus;
            this.routeIndex = entity.route.index;
        }

        @Override
        public Void apply(Simulation sim, Route.State entity) {
            entity.status = newStatus;
            return null;
        }

        @Override
        public Route.State getEntity(Simulation sim) {
            return sim.infraState.getRouteState(routeIndex);
        }

        @Override
        public String toString() {
            return String.format("RouteStatusChange { route: %d, status: %s }", routeIndex, newStatus);
        }
    }
}
