import { MfaMethod } from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import { useConfigureMfaStore } from '../hooks/useConfigureMfaStore';
import type { MfaVerificationMethod } from '../types';
import { ConfigureSelectMethodsStep } from './ConfigureSelectMethodsStep/ConfigureSelectMethodsStep';
import { ConfigureSelectVerificationStep } from './ConfigureSelectVerificationStep/ConfigureSelectVerificationStep';
import { ConfigureVerifyEmailStep } from './ConfigureVerifyEmailStep/ConfigureVerifyEmailStep';
import { ConfigureVerifyFido2Step } from './ConfigureVerifyFido2Step/ConfigureVerifyFido2Step';
import { ConfigureVerifyOidcStep } from './ConfigureVerifyOidcStep/ConfigureVerifyOidcStep';
import { ConfigureVerifyTotpStep } from './ConfigureVerifyTotpStep/ConfigureVerifyTotpStep';

type Props = {
  onCancel: () => void;
  onSessionExpired: () => void;
};

/** Picks what the wizard sets up, then verifies the session with an existing factor. */
export const ConfigureMfaVerify = ({ onCancel, onSessionExpired }: Props) => {
  const candidates = useConfigureMfaStore((s) => s.verificationMethods);
  const methodsSelected = useConfigureMfaStore((s) => isPresent(s.selectedMethods));
  const verificationMethod = useConfigureMfaStore((s) => s.verificationMethod);

  if (!methodsSelected) {
    return <ConfigureSelectMethodsStep onCancel={onCancel} />;
  }

  if (candidates.length > 1 && !isPresent(verificationMethod)) {
    return <ConfigureSelectVerificationStep />;
  }

  // for the type only, a session always offers a method or falls back to email
  const method: MfaVerificationMethod =
    verificationMethod ?? candidates[0] ?? MfaMethod.Email;

  switch (method) {
    case MfaMethod.Totp:
      return <ConfigureVerifyTotpStep onSessionExpired={onSessionExpired} />;
    case MfaMethod.Email:
      return <ConfigureVerifyEmailStep onSessionExpired={onSessionExpired} />;
    case MfaMethod.Fido2:
      return <ConfigureVerifyFido2Step onSessionExpired={onSessionExpired} />;
    case MfaMethod.Oidc:
      return <ConfigureVerifyOidcStep onSessionExpired={onSessionExpired} />;
  }
};
