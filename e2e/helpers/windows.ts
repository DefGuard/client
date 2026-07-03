import { browser } from '@wdio/globals';

export const switchToFullView = () =>
  browser.waitUntil(
    async () => {
      for (const handle of await browser.getWindowHandles()) {
        await browser.switchToWindow(handle);
        if ((await browser.getUrl()).includes('/full')) {
          return true;
        }
      }
      return false;
    },
    { timeoutMsg: 'No full view window found' },
  );
