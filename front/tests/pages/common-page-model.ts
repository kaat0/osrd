import { expect, type Locator, type Page } from '@playwright/test';

class CommonPage {
  readonly page: Page;

  readonly toastContainer: Locator;

  readonly toastTitle: Locator;

  readonly tagField: Locator;

  readonly viteOverlay: Locator;

  readonly toastSNCF: Locator;

  readonly closeToastButton: Locator;

  constructor(page: Page) {
    this.page = page;
    this.toastContainer = page.getByTestId('toast-SNCF');
    this.toastTitle = this.toastContainer.getByTestId('toast-SNCF-title');
    this.tagField = page.getByTestId('chips-input');
    this.viteOverlay = page.locator('vite-plugin-checker-error-overlay');
    this.toastSNCF = page.getByTestId('toast-SNCF');
    this.closeToastButton = page.getByTestId('close-toast-button');
  }

  // Set the tag of project, study or scenario
  async setTag(tag: string) {
    await this.tagField.fill(tag);
    await this.tagField.press('Enter');
  }

  // Verify the text of the last toast notification title
  async checkLastToastTitle(expectedText: string | RegExp): Promise<void> {
    await expect(this.toastTitle.last()).toHaveText(expectedText);
  }

  async removeViteOverlay() {
    if (await this.viteOverlay.count()) {
      await this.viteOverlay.evaluate((node) => node.setAttribute('style', 'display:none;'));
    }
  }

  async closeToastNotification(): Promise<void> {
    const closeToastElements = await this.closeToastButton.all();
    await Promise.all(closeToastElements.map((closeToastElement) => closeToastElement.click()));
  }
}

export default CommonPage;
