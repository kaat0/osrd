package fr.sncf.osrd.infra.routegraph;

import fr.sncf.osrd.utils.TrackSectionLocation;

public class RouteLocation {
    public final Route route;
    public final double offset;

    public RouteLocation(Route route, double offset) {
        this.route = route;
        this.offset = offset;
    }

    /** Return the track section location */
    public TrackSectionLocation getTrackSectionLocation() {
        double offsetLeft = offset;
        for (int i = 0; i < route.tvdSectionsPaths.size(); i++) {
            var tvdSectionPath = route.tvdSectionsPaths.get(i);
            var tvdSectionPathDir = route.tvdSectionsPathDirections.get(i);
            for (var trackSection : tvdSectionPath.getTrackSections(tvdSectionPathDir)) {
                if (trackSection.length() >= offsetLeft)
                    return new TrackSectionLocation(trackSection.edge, offsetLeft);
                offsetLeft -= trackSection.length();
            }
        }
        return null;
    }
}
