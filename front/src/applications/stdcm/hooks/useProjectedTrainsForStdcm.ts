import { useEffect, useMemo, useState } from 'react';

import { useSelector } from 'react-redux';

import type { TrainSpaceTimeData } from 'applications/operationalStudies/types';
import type { StdcmSuccessResponse } from 'applications/stdcm/types';
import { osrdEditoastApi } from 'common/api/osrdEditoastApi';
import { useInfraID, useOsrdConfSelectors } from 'common/osrdContext';
import useLazyProjectTrains from 'modules/simulationResult/components/SpaceTimeChart/useLazyProjectTrains';

import formatStdcmTrainIntoSpaceTimeData from '../utils/formatStdcmIntoSpaceTimeData';

const useProjectedTrainsForStdcm = (stdcmResponse?: StdcmSuccessResponse) => {
  const infraId = useInfraID();
  const { getTimetableID } = useOsrdConfSelectors();
  const timetableId = useSelector(getTimetableID);

  const [spaceTimeData, setSpaceTimeData] = useState<TrainSpaceTimeData[]>([]);
  const [trainIdsToProject, setTrainIdsToProject] = useState<Set<number>>(new Set());

  const { data: timetable } = osrdEditoastApi.endpoints.getTimetableById.useQuery(
    { id: timetableId! },
    {
      skip: !timetableId,
    }
  );

  const trainIds = useMemo(() => timetable?.train_ids || [], [timetable]);
  const { currentData: trainSchedules } = osrdEditoastApi.endpoints.postTrainSchedule.useQuery(
    {
      body: {
        ids: trainIds,
      },
    },
    {
      skip: !trainIds.length,
    }
  );

  const stdcmProjectedTrain = useMemo(
    () => (stdcmResponse ? formatStdcmTrainIntoSpaceTimeData(stdcmResponse) : undefined),
    [stdcmResponse]
  );

  const allTrainsProjected = useMemo(() => trainIdsToProject.size === 0, [trainIdsToProject]);

  const { projectedTrainsById: projectedTimetableTrainsById } = useLazyProjectTrains({
    infraId,
    trainIdsToProject,
    path: stdcmResponse?.path,
    trainSchedules,
    setTrainIdsToProject,
  });

  useEffect(() => {
    const newSpaceTimeData = Array.from(projectedTimetableTrainsById.values());

    if (stdcmProjectedTrain) {
      newSpaceTimeData.push(stdcmProjectedTrain);
    }

    setSpaceTimeData(newSpaceTimeData);
  }, [stdcmProjectedTrain, projectedTimetableTrainsById]);

  if (!infraId || !stdcmResponse) return null;

  return {
    spaceTimeData,
    projectionLoaderData: { allTrainsProjected, totalTrains: trainIds.length },
  };
};

export default useProjectedTrainsForStdcm;
