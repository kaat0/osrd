package fr.sncf.osrd.stdcm.graph

import com.google.common.primitives.Doubles.min
import kotlin.Double.Companion.POSITIVE_INFINITY
import kotlin.math.max

/**
 * This class combines all the time-related elements of a node/edge.
 *
 * It contains data about the possible times that can be covered, as well as extra context about
 * stop times, time since departure, when the closest conflicts are, and so on.
 *
 * Some node-specific or edge-specific values and getters are left in their specific classes.
 *
 * Note: stop duration are described as mutable, but this feature isn't implemented yet.
 */
data class TimeData(
    /**
     * Earliest time when we can enter the current location. For edges, this is the entry time. This
     * is *not* the time at which the train *will* reach this location: we can delay departure times
     * or add allowances further on the path.
     */
    val earliestReachableTime: Double,

    /**
     * We can delay departure time from either the train start or stops. This value represents how
     * much more delay we can add to the last departure without causing any conflict. The delay
     * would be added to the departure time of the last stop, or to the global departure time. We
     * first delay the departure time whenever possible, then lengthen stop durations if it's not
     * enough.
     */
    val maxDepartureDelayingWithoutConflict: Double,

    /**
     * This value describes how much delay we can add to the train departure time without causing
     * any conflict (from the departure to the current point). This is the preferred method of
     * delaying when the train reaches the current point.
     */
    val maxFirstDepartureDelaying: Double,

    /**
     * Time of the next conflict on the given location. Used both to identify edges that go through
     * the same "opening", and to figure out how much delay we can add locally (with margins).
     */
    val timeOfNextConflictAtLocation: Double,

    /**
     * Time the train has spent moving since its departure. Does not include stop times. Does not
     * account for engineering allowances that would be added further down the path.
     */
    val totalRunningTime: Double,

    /**
     * Current estimation of the departure time, may be delayed further down the path (up to the
     * first stop).
     */
    val departureTime: Double,

    /**
     * List of stop data over the path up to the current point. The duration of the last stop may be
     * retroactively lengthened further down the path.
     */
    val stopTimeData: List<StopTimeData>,

    /**
     * Global delay that have been added to avoid conflicts on the given element, by delaying the
     * last departure. This is the value that is added on this specific node/edge.
     */
    val delayAddedToLastDeparture: Double = 0.0,
) {
    /**
     * Time elapsed since the departure time. This may be changed further down the path by
     * lengthening stop durations or adding engineering allowances.
     */
    val timeSinceDeparture = totalRunningTime + stopTimeData.sumOf { it.currentDuration }

    val totalStopDuration = stopTimeData.sumOf { it.currentDuration }

    /** Returns a copy of the current instance, with added travel / stop time. */
    fun withAddedTime(
        extraTravelTime: Double,
        extraStopTime: Double?,
        maxAdditionalStopTime: Double?
    ): TimeData {
        assert((extraStopTime == null) == (maxAdditionalStopTime == null)) {
            "Can't set just one of 'stop duration' or 'max additional stop duration' without the other"
        }
        var newStopData = stopTimeData
        var maxDepartureDelayingWithoutConflict = maxDepartureDelayingWithoutConflict
        val nextEarliestReachableTime =
            earliestReachableTime + extraTravelTime + (extraStopTime ?: 0.0)
        var timeOfNextConflict = timeOfNextConflictAtLocation
        if (maxAdditionalStopTime != null)
            timeOfNextConflict = nextEarliestReachableTime + maxAdditionalStopTime
        if (extraStopTime != null) {
            val stopDataCopy = newStopData.toMutableList()
            stopDataCopy.add(
                StopTimeData(
                    currentDuration = extraStopTime,
                    minDuration = extraStopTime,
                    maxDepartureDelayBeforeStop = maxDepartureDelayingWithoutConflict,
                )
            )
            newStopData = stopDataCopy
            maxDepartureDelayingWithoutConflict = maxAdditionalStopTime!!
        }
        return copy(
            earliestReachableTime = nextEarliestReachableTime,
            totalRunningTime = totalRunningTime + extraTravelTime,
            stopTimeData = newStopData,
            maxDepartureDelayingWithoutConflict = maxDepartureDelayingWithoutConflict,
            maxFirstDepartureDelaying =
                min(maxFirstDepartureDelaying, (maxAdditionalStopTime ?: POSITIVE_INFINITY)),
            timeOfNextConflictAtLocation = timeOfNextConflict,
        )
    }

    /**
     * Return a copy of the current instance, with "shifted" time values. Used to create new edges.
     * The shift can be made by delaying the last departure (either by lengthening the last stop if
     * any, or by making the train start at a later time). Any remaining time delta is considered to
     * be reached by making the train run slower (with engineering allowances).
     *
     * @param timeShift by how much we delay the new element compared to the earliest possible time.
     *   A value > 0 means that we want to arrive later (to avoid a conflict)
     * @param delayAddedToLastDeparture how much extra delay we add to the last departure (train
     *   start time or last stop). This is one method to reach `timeShift`. The remaining difference
     *   is counted as extra running time.
     * @param timeOfNextConflictAtLocation when is the first conflict at the given location
     * @param maxDepartureDelayingWithoutConflict how much delay we can add at the given location
     *   without causing conflict
     */
    fun shifted(
        timeShift: Double,
        delayAddedToLastDeparture: Double,
        timeOfNextConflictAtLocation: Double,
        maxDepartureDelayingWithoutConflict: Double,
    ): TimeData {
        assert(timeShift >= delayAddedToLastDeparture)
        var newStopData = stopTimeData
        var newDepartureTime = departureTime
        var newMaxFirstDepartureDelaying = maxFirstDepartureDelaying
        if (delayAddedToLastDeparture > 0) {
            val firstDepartureTimeDelay = min(maxFirstDepartureDelaying, delayAddedToLastDeparture)
            val lastStopExtraDuration = delayAddedToLastDeparture - firstDepartureTimeDelay
            newDepartureTime += firstDepartureTimeDelay
            newMaxFirstDepartureDelaying -= firstDepartureTimeDelay
            if (newStopData.isNotEmpty()) {
                val stopDataCopy = newStopData.toMutableList()
                val oldLastStopData = stopDataCopy.last()
                stopDataCopy[stopDataCopy.size - 1] =
                    oldLastStopData.withAddedStopTime(lastStopExtraDuration)
                newStopData = stopDataCopy
            }
        }
        newMaxFirstDepartureDelaying =
            min(newMaxFirstDepartureDelaying, maxDepartureDelayingWithoutConflict)
        val extraRunningTime = max(0.0, timeShift - delayAddedToLastDeparture)
        return copy(
            earliestReachableTime = earliestReachableTime + timeShift,
            maxDepartureDelayingWithoutConflict = maxDepartureDelayingWithoutConflict,
            timeOfNextConflictAtLocation = timeOfNextConflictAtLocation,
            delayAddedToLastDeparture = delayAddedToLastDeparture,
            stopTimeData = newStopData,
            departureTime = newDepartureTime,
            totalRunningTime = totalRunningTime + extraRunningTime,
            maxFirstDepartureDelaying = newMaxFirstDepartureDelaying,
        )
    }

    /**
     * When we have moved further down the path, this method gives an updated estimation of the
     * earliest reachable time. This accounts for any extra delay that may have been added to any
     * departure time. "updatedTimeData" is the latest available TimeData instance.
     */
    fun getUpdatedEarliestReachableTime(updatedTimeData: TimeData): Double {
        val addedDepartureDelay = updatedTimeData.departureTime - departureTime

        // Ignore stops that haven't been reached in the current instance
        val updatedStopValues = updatedTimeData.stopTimeData.subList(0, stopTimeData.size)
        val addedStopValues =
            (updatedStopValues zip stopTimeData).map {
                it.first.currentDuration - it.second.currentDuration
            }

        assert(addedDepartureDelay >= 0)
        assert(addedStopValues.all { it >= 0 })

        return earliestReachableTime + addedDepartureDelay + addedStopValues.sum()
    }
}

data class StopTimeData(
    /** Current stop duration. It may be made longer further down the path. */
    val currentDuration: Double,
    /** Minimum stop duration as described in the input. */
    val minDuration: Double,
    /** We need to keep track of how much delay we can add before this stop. */
    val maxDepartureDelayBeforeStop: Double
) {
    fun withAddedStopTime(extraStopTime: Double): StopTimeData {
        return copy(currentDuration = currentDuration + extraStopTime)
    }
}
