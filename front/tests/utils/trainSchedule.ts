import type { APIResponse, APIRequestContext } from '@playwright/test';

import type { TrainScheduleResult } from 'common/api/osrdEditoastApi';

import { getApiContext, postApiRequest } from './api-setup';
import { handleApiResponse } from './index';

// Function to import train schedules and return the API response
export async function sendTrainSchedules(
  timetableId: number,
  body: object
): Promise<TrainScheduleResult[]> {
  const apiContext: APIRequestContext = await getApiContext();
  const trainSchedulesResponse: APIResponse = await apiContext.post(
    `/api/v2/timetable/${timetableId}/train_schedule/`,
    {
      data: JSON.stringify(body),
      headers: {
        'Content-Type': 'application/json',
      },
    }
  );
  handleApiResponse(trainSchedulesResponse, 'Failed to send train schedule');
  return trainSchedulesResponse.json();
}

// Function to extract train IDs from the train schedules
export function getTrainIds(trainSchedules: TrainScheduleResult[]) {
  return trainSchedules.map((item: TrainScheduleResult) => item.id);
}
// Function to post simulation using extracted train IDs
export async function postSimulation(
  response: TrainScheduleResult[],
  infraId: number
): Promise<void> {
  const trainIds = getTrainIds(response);
  if (trainIds.length > 0) {
    await postApiRequest(
      `/api/v2/train_schedule/simulation_summary/`,
      { ids: trainIds, infra_id: infraId },
      undefined,
      'Failed to post simulation'
    );
  }
}
