package fr.sncf.osrd.geom;

import static fr.sncf.osrd.geom.WGS84Interpolator.EARTH_RADIUS;
import static java.lang.Math.*;

public record Point(
        // Longitude
        double lat,
        // Latitude
        double lon) {

    /**
     * Returns the distance between this point and another in meters. Uses equirectangular distance approximation (very fast but not 100% accurate)
     */
    public double distanceAsMeters(Point other) {
        var lon1 = toRadians(lon);
        var lon2 = toRadians(other.lon);
        var lat1 = toRadians(lat);
        var lat2 = toRadians(other.lat);
        var xDiff = (lon1 - lon2) * cos(0.5 * (lat1 + lat2));
        var yDiff = lat1 - lat2;
        return EARTH_RADIUS * sqrt(xDiff * xDiff + yDiff * yDiff);
    }

    @Override
    public String toString() {
        return String.format("{lat=%f, lon=%f}", lat, lon);
    }
}
