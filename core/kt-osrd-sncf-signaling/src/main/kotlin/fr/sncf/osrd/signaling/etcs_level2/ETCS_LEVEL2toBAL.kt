package fr.sncf.osrd.signaling.etcs_level2

import fr.sncf.osrd.reporting.exceptions.ErrorType
import fr.sncf.osrd.reporting.exceptions.OSRDError
import fr.sncf.osrd.signaling.*
import fr.sncf.osrd.sim_infra.api.SigParameters
import fr.sncf.osrd.sim_infra.api.SigSettings
import fr.sncf.osrd.sim_infra.api.SigState
import fr.sncf.osrd.sim_infra.api.SigStateSchema

object ETCS_LEVEL2toBAL : SignalDriver {
    override val name = "ETCS_LEVEL2-BAL"
    override val inputSignalingSystem = "ETCS_LEVEL2"
    override val outputSignalingSystem = "BAL"

    private fun cascadePrimaryAspect(aspect: String): String {
        return when (aspect) {
            "300VL" -> "VL"
            "300(VL)" -> "VL"
            "270A" -> "VL"
            "220A" -> "VL"
            "160A" -> "VL"
            "080A" -> "VL"
            "000" -> "A"
            "OCCUPIED" -> "C"
            else -> throw OSRDError.newAspectError(aspect)
        }
    }

    override fun evalSignal(
        signal: SigSettings,
        parameters: SigParameters,
        stateSchema: SigStateSchema,
        maView: MovementAuthorityView?,
        limitView: SpeedLimitView?
    ): SigState {
        return stateSchema {
            assert(maView!!.hasNextSignal)
            when (maView.protectionStatus) {
                ProtectionStatus.NO_PROTECTED_ZONES ->
                    throw OSRDError(ErrorType.BALUnprotectedZones)
                ProtectionStatus.INCOMPATIBLE -> value("aspect", "C")
                ProtectionStatus.OCCUPIED -> value("aspect", "S")
                ProtectionStatus.CLEAR -> {
                    val nextSignalState = maView.nextSignalState
                    val nextAspect = nextSignalState.getEnum("aspect")
                    value("aspect", cascadePrimaryAspect(nextAspect))
                }
            }
        }
    }

    override fun checkSignal(reporter: SignalDiagReporter, signal: SigSettings, block: SigBlock) {}
}
