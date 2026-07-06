import { $, browser } from '@wdio/globals';
import { submitTotpCode } from './mfa.js';
import { canPingGateway } from './tunnel.js';

export const connectAndPing = async (totpSecret?: string) => {
  const connect = $('.connect-button');
  await connect.waitForClickable();
  await connect.click();

  if (totpSecret) {
    const view = $('#mfa-totp-view');
    await view.waitForDisplayed();
    await submitTotpCode(
      totpSecret,
      '#mfa-totp-view',
      async () => {
        const verify = view.$('button=Verify');
        await verify.waitForClickable();
        await verify.click();
      },
      async () => !(await view.isDisplayed().catch(() => false)),
    );
  }

  await browser.waitUntil(() => canPingGateway(), {
    timeout: 30_000,
    interval: 2_000,
    timeoutMsg: 'Could not ping the gateway through the VPN',
  });
};
