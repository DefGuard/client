import { $, browser } from '@wdio/globals';

export const switchToFullView = async () => {
  for (const handle of await browser.getWindowHandles()) {
    await browser.switchToWindow(handle);
    const url = await browser.getUrl();
    if (url.includes('/full')) {
      return;
    }
    if (url.includes('/compact')) {
      await browser.url('tauri://localhost/full/');
      return;
    }
  }
  throw new Error('No full view window found');
};

export const switchToTrayView = async () => {
  await switchToFullView();
  await browser.url('tauri://localhost/compact/');
  await $('#compact-locations-page').waitForDisplayed();
};
