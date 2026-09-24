import { useMemo } from 'react';
import { MfaMethod } from '../../../../shared/rust-api/types';
import { isPresent } from '../../../../shared/utils/isPresent';
import { useConfigureMfaStore } from '../hooks/useConfigureMfaStore';
import type { MfaVerificationMethod } from '../types';
import { verificationMethodsOf } from '../utils';
import { ConfigureSelectMethodsStep } from './ConfigureSelectMethodsStep/ConfigureSelectMethodsStep';
import { ConfigureSelectVerificationStep } from './ConfigureSelectVerificationStep/ConfigureSelectVerificationStep';
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
  const verificationMethod = useConfigureMfaStore((s) => s.verificationMethod);

  const candidates = useMemo(
    () => verificationMethodsOf(configuredMethods),
    [configuredMethods],
  );

  if (!methodsSelected) {
    return <ConfigureSelectMethodsStep onCancel={onCancel} />;
  }

  if (candidates.length > 1 && !isPresent(verificationMethod)) {
    return <ConfigureSelectVerificationStep />;
  }

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
  }
};
