import type {
  GetLightRollingStockApiResponse,
  LightRollingStock,
} from 'common/api/osrdEditoastApi';

import {
  getApiRequest,
  deleteApiRequest,
  getInfra,
  getStudy,
  getScenario,
  getProject,
} from './api-setup';
import { logger } from '../test-logger';

/**
 * Delete infrastructure by name if it exists.
 *
 * @param infraName - The name of the infrastructure to delete.
 * @returns {Promise<void>} - A promise that resolves when the infrastructure is deleted or if not found.
 */
export async function deleteInfra(infraName: string): Promise<void> {
  const infra = await getInfra(infraName);

  if (infra) {
    await deleteApiRequest(`/api/infra/${infra.id}`);
  } else {
    logger.warn(`Infra "${infraName}" not found for deletion.`);
  }
}

/**
 * Delete a project by name if it exists.
 *
 * @param projectName - The name of the project to delete.
 * @returns {Promise<void>} - A promise that resolves when the project is deleted or if not found.
 */
export async function deleteProject(projectName: string): Promise<void> {
  const project = await getProject(projectName);
  if (project) {
    await deleteApiRequest(`/api/projects/${project.id}`);
  } else {
    logger.warn(`Project "${projectName}" not found for deletion.`);
  }
}

/**
 * Delete rolling stocks by their names if they exist.
 *
 * @param rollingStockNames - The list of rolling stock names to delete.
 * @returns {Promise<void>} - A promise that resolves when the matching rolling stocks are deleted or if none are found.
 */
export async function deleteRollingStocks(rollingStockNames: string[]): Promise<void> {
  const rollingStocks: GetLightRollingStockApiResponse = await getApiRequest(
    '/api/light_rolling_stock',
    { page_size: 500 }
  );

  // Find rolling stocks that match the provided names
  const rollingStockIds = rollingStocks.results
    .filter((r: LightRollingStock) => rollingStockNames.includes(r.name))
    .map((r: LightRollingStock) => r.id);

  if (rollingStockIds.length > 0) {
    // Delete each rolling stock by ID
    await Promise.all(
      rollingStockIds.map((id: number) => deleteApiRequest(`/api/rolling_stock/${id}`))
    );
  } else {
    logger.warn('No matching rolling stocks found for deletion.');
  }
}

/**
 * Delete a study by name if it exists.
 *
 * @param studyName - The name of the study to delete.
 * @returns {Promise<void>} - A promise that resolves when the study is deleted or if not found.
 */
export async function deleteStudy(projectId: number, studyName: string): Promise<void> {
  const study = await getStudy(projectId, studyName);

  if (study) {
    await deleteApiRequest(`/api/projects/${projectId}/studies/${study.id}`);
  } else {
    logger.warn(`Study "${studyName}" not found for deletion.`);
  }
}

/**
 * Delete a scenario by name if it exists.
 *
 * @param scenarioName - The name of the scenario to delete.
 * @returns {Promise<void>} - A promise that resolves when the scenario is deleted or if not found.
 */
export async function deleteScenario(
  projectId: number,
  studyId: number,
  scenarioName: string
): Promise<void> {
  const scenario = await getScenario(projectId, studyId, scenarioName);
  if (scenario?.id) {
    await deleteApiRequest(
      `/api/projects/${projectId}/studies/${studyId}/scenarios/${scenario.id}`
    );
  } else {
    logger.warn(`Scenario "${scenarioName}" not found for deletion.`);
  }
}
