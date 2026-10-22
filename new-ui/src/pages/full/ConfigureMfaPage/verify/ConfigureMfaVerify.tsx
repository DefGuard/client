import { MfaMethod } from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import { useConfigureMfaStore } from '../hooks/useConfigureMfaStore';
import type { MfaVerificationMethod } from '../types';
import { showsVerificationPicker } from '../utils';
import { ConfigureSelectVerificationStep } from './ConfigureSelectVerificationStep/ConfigureSelectVerificationStep';
import { ConfigureVerifyEmailStep } from './ConfigureVerifyEmailStep/ConfigureVerifyEmailStep';
import { ConfigureVerifyFido2Step } from './ConfigureVerifyFido2Step/ConfigureVerifyFido2Step';
import { ConfigureVerifyOidcStep } from './ConfigureVerifyOidcStep/ConfigureVerifyOidcStep';
import { ConfigureVerifyTotpStep } from './ConfigureVerifyTotpStep/ConfigureVerifyTotpStep';

type Props = {
  onCancel: () => void;
  onSessionExpired: () => void;
};

/** Verifies the session with an existing factor before anything can be picked to set up. */
export const ConfigureMfaVerify = ({ onCancel, onSessionExpired }: Props) => {
  const candidates = useConfigureMfaStore((s) => s.verificationMethods);
  const verificationMethod = useConfigureMfaStore((s) => s.verificationMethod);

  if (showsVerificationPicker(candidates) && !isPresent(verificationMethod)) {
    return <ConfigureSelectVerificationStep onCancel={onCancel} />;
  }

  // for the type only, a session with nothing to verify with is refused at start
  const method: MfaVerificationMethod =
    verificationMethod ?? candidates[0] ?? MfaMethod.Email;

  switch (method) {
    case MfaMethod.Totp:
      return (
        <ConfigureVerifyTotpStep
          onCancel={onCancel}
          onSessionExpired={onSessionExpired}
        />
      );
    case MfaMethod.Email:
      return (
        <ConfigureVerifyEmailStep
          onCancel={onCancel}
          onSessionExpired={onSessionExpired}
        />
      );
    case MfaMethod.Fido2:
      return (
        <ConfigureVerifyFido2Step
          onCancel={onCancel}
          onSessionExpired={onSessionExpired}
        />
      );
    case MfaMethod.Oidc:
      return (
        <ConfigureVerifyOidcStep
          onCancel={onCancel}
          onSessionExpired={onSessionExpired}
        />
      );
  }
};
