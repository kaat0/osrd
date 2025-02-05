package fr.sncf.osrd.railjson.schema.rollingstock;

import com.squareup.moshi.Json;
import com.squareup.moshi.JsonAdapter;
import com.squareup.moshi.Moshi;
import fr.sncf.osrd.railjson.schema.common.Identified;
import java.util.Map;

public class RJSRollingStock implements Identified {
    public static final JsonAdapter<RJSRollingStock> adapter =
            new Moshi.Builder().add(RJSRollingResistance.adapter).build().adapter(RJSRollingStock.class);

    public static final transient String CURRENT_VERSION = "3.2";

    /** The version of the rolling stock format used */
    @Json(name = "railjson_version")
    public String railjsonVersion = null;

    /** A unique train identifier */
    public String name = null;

    /**
     * Engineers measured a number of effort curves for each rolling stock. These are referenced
     * from effort curve profiles. Effort curves associate a speed to a traction force.
     * https://en.wikipedia.org/wiki/Tractive_force#Tractive_effort_curves This match the default
     * effort curve to take
     */
    @Json(name = "effort_curves")
    public RJSEffortCurves effortCurves;

    @Json(name = "power_restrictions")
    public Map<String, String> powerRestrictions;

    /** The class of power usage of the train */
    @Json(name = "base_power_class")
    public String basePowerClass = null;

    /** the length of the train, in meters. */
    public double length = Double.NaN;

    /** The max speed of the train, in meters per seconds. */
    @Json(name = "max_speed")
    public double maxSpeed = Double.NaN;

    /**
     * The time the train takes to start up, in seconds. During this time, the train's maximum
     * acceleration is limited.
     */
    @Json(name = "startup_time")
    public double startUpTime = Double.NaN;

    /** The acceleration to apply during the startup state. */
    @Json(name = "startup_acceleration")
    public double startUpAcceleration = Double.NaN;

    /** The maximum acceleration when the train is in its regular operating mode. */
    @Json(name = "comfort_acceleration")
    public double comfortAcceleration = Double.NaN;

    /** The constant gamma braking coefficient used when NOT circulating under ETCS/ERTMS signaling system in m/s^2 */
    @Json(name = "const_gamma")
    public double constGamma = Double.NaN;

    /**
     * Inertia coefficient. The mass alone isn't sufficient to compute accelerations, as the wheels
     * and internals also need force to get spinning. This coefficient can be used to account for
     * the difference. It's without unit: effective mass = mass * inertia coefficient
     */
    @Json(name = "inertia_coefficient")
    public double inertiaCoefficient = Double.NaN;

    /** The mass of the train */
    public double mass = Double.NaN;

    @Json(name = "rolling_resistance")
    public RJSRollingResistance rollingResistance = null;

    @Json(name = "loading_gauge")
    public RJSLoadingGaugeType loadingGauge = null;

    @Json(name = "electrical_power_startup_time")
    public Double electricalPowerStartUpTime = null;

    @Json(name = "raise_pantograph_time")
    public Double raisePantographTime = null;

    @Json(name = "supported_signaling_systems")
    public String[] supportedSignalingSystems = new String[0];

    @Override
    public String getID() {
        return name;
    }
}
