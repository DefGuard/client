import type { UseNavigateResult } from '@tanstack/react-router';
import type { ConfigureFactorsPayload } from '../../../../shared/rust-api/types';
import { reportMfaConfigStartError } from './reportMfaConfigStartError';
import { discardMfaConfiguration, startMfaConfiguration } from './useConfigureMfaStore';

type Deps = {
  navigate: UseNavigateResult<string>;
};

/** What the full view does with a `configure-factors-trigger`. The route is only entered once
 *  the session exists, so a failure leaves the user where they were with a snackbar. */
export const runConfigureFactorsRequest = async (
  payload: ConfigureFactorsPayload,
  { navigate }: Deps,
): Promise<void> => {
  // A session an earlier run walked away from would otherwise outlive this one on the proxy.
  await discardMfaConfiguration();

  try {
    await startMfaConfiguration(payload.instance, {
      preselectedMethods: payload.methods,
      source: payload.source,
      location: payload.location,
    });
  } catch (err) {
    reportMfaConfigStartError(err);
    return;
  }

  await navigate({ to: '/full/configure-mfa' });
};
