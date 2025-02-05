import { useTranslation } from 'react-i18next';
import { useSelector } from 'react-redux';

import { MANAGE_TRAIN_SCHEDULE_TYPES } from 'applications/operationalStudies/consts';
import { osrdEditoastApi, type TrainScheduleResult } from 'common/api/osrdEditoastApi';
import { useStoreDataForRollingStockSelector } from 'modules/rollingStock/components/RollingStockSelector/useStoreDataForRollingStockSelector';
import checkCurrentConfig from 'modules/trainschedule/components/ManageTrainSchedule/helpers/checkCurrentConfig';
import { setFailure, setSuccess } from 'reducers/main';
import {
  getName,
  getStartTime,
  getOperationalStudiesConf,
} from 'reducers/osrdconf/operationalStudiesConf/selectors';
import { updateSelectedTrainId } from 'reducers/simulationResults';
import { useAppDispatch } from 'store';
import { formatToIsoDate } from 'utils/date';
import { castErrorToFailure } from 'utils/error';

import formatTrainSchedulePayload from '../helpers/formatTrainSchedulePayload';

const useUpdateTrainSchedule = (
  setIsWorking: (isWorking: boolean) => void,
  setDisplayTrainScheduleManagement: (type: string) => void,
  upsertTrainSchedules: (trainSchedules: TrainScheduleResult[]) => void,
  setTrainIdToEdit: (trainIdToEdit?: number) => void,
  dtoImport: () => void,
  trainIdToEdit?: number
) => {
  const { t } = useTranslation(['operationalStudies/manageTrainSchedule']);
  const [putTrainScheduleById] = osrdEditoastApi.endpoints.putTrainScheduleById.useMutation();
  const dispatch = useAppDispatch();
  const confName = useSelector(getName);
  const simulationConf = useSelector(getOperationalStudiesConf);
  const startTime = useSelector(getStartTime);
  const { rollingStock } = useStoreDataForRollingStockSelector();

  return async function submitConfUpdateTrainSchedules() {
    const formattedSimulationConf = checkCurrentConfig(
      simulationConf,
      t,
      dispatch,
      rollingStock?.name
    );

    if (formattedSimulationConf && trainIdToEdit) {
      setIsWorking(true);
      const trainSchedule = formatTrainSchedulePayload(
        formattedSimulationConf,
        confName,
        startTime
      );
      try {
        const trainScheduleResult = await putTrainScheduleById({
          id: trainIdToEdit,
          trainScheduleForm: trainSchedule,
        }).unwrap();
        upsertTrainSchedules([trainScheduleResult]);
        dtoImport();
        dispatch(
          setSuccess({
            title: t('trainUpdated'),
            text: `${confName}: ${formatToIsoDate(startTime, true)}`,
          })
        );
        dispatch(updateSelectedTrainId(trainIdToEdit));
        setDisplayTrainScheduleManagement(MANAGE_TRAIN_SCHEDULE_TYPES.none);
        setTrainIdToEdit(undefined);
      } catch (e) {
        dispatch(setFailure(castErrorToFailure(e)));
      } finally {
        setIsWorking(false);
      }
    }
  };
};

export default useUpdateTrainSchedule;
