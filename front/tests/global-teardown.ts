import { test as teardown } from '@playwright/test';

import ROLLING_STOCK_NAMES, { globalProjectName, stdcmProjectName } from './assets/project-const';
import { deleteProject, deleteRollingStocks } from './utils/teardown-utils';

teardown('teardown', async () => {
  try {
    await Promise.all([
      await deleteProject(stdcmProjectName),
      await deleteProject(globalProjectName),
      deleteRollingStocks(ROLLING_STOCK_NAMES),
    ]);
    console.info('Test data teardown completed successfully.');
  } catch (error) {
    console.error('Error during test data teardown:', error);
  }
});
