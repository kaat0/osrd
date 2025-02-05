import { compact, isNaN, isNil } from 'lodash';

import type { TrainScheduleBase } from 'common/api/osrdEditoastApi';
import type { PathStep } from 'reducers/osrdconf/types';
import { formatDurationAsISO8601 } from 'utils/timeManipulation';

const formatSchedule = (pathSteps: PathStep[]): TrainScheduleBase['schedule'] => {
  const schedules = pathSteps.map((step) => {
    if (step?.arrival || step.stopFor) {
      return {
        at: step.id,
        arrival: step.arrival?.toISOString() ?? undefined,
        locked: step.locked,
        reception_signal: step.receptionSignal,
        stop_for:
          isNil(step.stopFor) || isNaN(Number(step.stopFor))
            ? undefined
            : formatDurationAsISO8601(Number(step.stopFor)),
      };
    }
    return undefined;
  });
  return compact(schedules);
};

export default formatSchedule;
