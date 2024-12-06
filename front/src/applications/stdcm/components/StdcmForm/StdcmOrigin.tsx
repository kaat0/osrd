import { useMemo } from 'react';

import { useTranslation } from 'react-i18next';
import { useSelector } from 'react-redux';

import OriginIcon from 'assets/pictures/mapMarkers/start.svg';
import { useOsrdConfActions, useOsrdConfSelectors } from 'common/osrdContext';
import type { StdcmConfSliceActions } from 'reducers/osrdconf/stdcmConf';
import type { StdcmConfSelectors } from 'reducers/osrdconf/stdcmConf/selectors';
import { useAppDispatch } from 'store';
import { extractDateAndTimefromISO, generateISODateFromDateTime } from 'utils/date';

import StdcmCard from './StdcmCard';
import StdcmOperationalPoint from './StdcmOperationalPoint';
import StdcmOpSchedule from './StdcmOpSchedule';
import type { ArrivalTimeTypes, ScheduleConstraint, StdcmConfigCardProps } from '../../types';

const StdcmOrigin = ({ disabled = false }: StdcmConfigCardProps) => {
  const { t } = useTranslation('stdcm');
  const dispatch = useAppDispatch();

  const { getStdcmOrigin } = useOsrdConfSelectors() as StdcmConfSelectors;
  const origin = useSelector(getStdcmOrigin);

  const { updateStdcmPathStep } = useOsrdConfActions() as StdcmConfSliceActions;

  const { originArrival, originToleranceValues } = useMemo(
    () => ({
      originArrival: origin.arrival ? extractDateAndTimefromISO(origin.arrival) : undefined,
      originToleranceValues: {
        arrivalToleranceBefore: origin.tolerances!.before,
        arrivalToleranceAfter: origin.tolerances!.after,
      },
    }),
    [origin]
  );

  const onOriginArrivalChange = (schedule: ScheduleConstraint) => {
    dispatch(
      updateStdcmPathStep({
        id: origin.id,
        updates: { arrival: generateISODateFromDateTime(schedule) },
      })
    );
  };

  const onOriginArrivalTypeChange = (arrivalType: ArrivalTimeTypes) => {
    dispatch(
      updateStdcmPathStep({
        id: origin.id,
        updates: { arrivalType },
      })
    );
  };

  const onOriginToleranceChange = ({
    toleranceBefore,
    toleranceAfter,
  }: {
    toleranceBefore: number;
    toleranceAfter: number;
  }) => {
    dispatch(
      updateStdcmPathStep({
        id: origin.id,
        updates: { tolerances: { before: toleranceBefore, after: toleranceAfter } },
      })
    );
  };

  return (
    <StdcmCard
      name={t('trainPath.origin')}
      title={<img src={OriginIcon} alt="origin" className="stdcm-origin-icon" />}
      className="extremity"
      disabled={disabled}
      hasTip
    >
      {'uic' in origin && (
        <StdcmOperationalPoint point={origin} opPointId={origin.id} disabled={disabled} />
      )}
      <StdcmOpSchedule
        onArrivalChange={onOriginArrivalChange}
        onArrivalTypeChange={onOriginArrivalTypeChange}
        onArrivalToleranceChange={onOriginToleranceChange}
        opTimingData={originArrival}
        opToleranceValues={originToleranceValues}
        opScheduleTimeType={origin.arrivalType}
        disabled={disabled}
        opId="origin-arrival"
      />
    </StdcmCard>
  );
};

export default StdcmOrigin;
