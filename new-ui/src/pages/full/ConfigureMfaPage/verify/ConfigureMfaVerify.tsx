import { MfaMethod } from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import { useConfigureMfaStore } from '../hooks/useConfigureMfaStore';
import { MFA_VERIFICATION_METHODS, type MfaVerificationMethod } from '../types';
import { ConfigureSelectMethodsStep } from './ConfigureSelectMethodsStep/ConfigureSelectMethodsStep';
import { ConfigureVerifyEmailStep } from './ConfigureVerifyEmailStep/ConfigureVerifyEmailStep';
import { ConfigureVerifyTotpStep } from './ConfigureVerifyTotpStep/ConfigureVerifyTotpStep';

type Props = {
  onCancel: () => void;
  onSessionExpired: () => void;
};

/** Picks what the wizard sets up, then verifies the session with an existing factor. */
export const ConfigureMfaVerify = ({ onCancel, onSessionExpired }: Props) => {
  const configuredMethods = useConfigureMfaStore((s) => s.configuredMethods);
  const methodsSelected = useConfigureMfaStore((s) => isPresent(s.selectedMethods));

  const method: MfaVerificationMethod =
    MFA_VERIFICATION_METHODS.find((candidate) => configuredMethods.includes(candidate)) ??
    MfaMethod.Email;

  if (!methodsSelected) {
    return <ConfigureSelectMethodsStep onCancel={onCancel} />;
  }

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
  }
};
