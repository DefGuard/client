import { $, browser } from '@wdio/globals';
import { submitTotpCode } from './mfa.js';
import { canPingGateway } from './tunnel.js';

export const connectAndPing = async (totpSecret?: string) => {
  const connect = $('.connect-button');
  await connect.waitForClickable();
  await connect.click();

  if (totpSecret) {
    await $('#mfa-totp-view').waitForDisplayed();
    await submitTotpCode(
      totpSecret,
      '#mfa-totp-view',
      async () => {
        const verify = $('#mfa-totp-view').$('button=Verify');
        await verify.waitForClickable();
        await verify.click();
      },
      () => $('#mfa-totp-view').isDisplayed().then((shown) => !shown, () => true),
    );
  }

  await browser.waitUntil(() => canPingGateway(), {
    timeout: 30_000,
    interval: 2_000,
    timeoutMsg: 'Could not ping the gateway through the VPN',
  });
};
