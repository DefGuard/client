import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { Fragment, useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { CodeInput } from '../../../../../shared/components/CodeInput/CodeInput';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { CopyField } from '../../../../../shared/components/CopyField/CopyField';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { QrCard } from '../../../../../shared/components/QrCard/QrCard';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { api } from '../../../../../shared/rust-api/api';
import { MfaMethod } from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import {
  discardMfaConfiguration,
  selectPendingMethod,
  useConfigureMfaStore,
} from '../../hooks/useConfigureMfaStore';
import { useMfaConfigErrorHandler } from '../../hooks/useMfaConfigErrorHandler';
import { ConfigureMfaStep } from '../../types';

const CODE_LENGTH = 6;

interface Props {
  onCancel: () => void;
  /** The session outlived its deadline, so the whole flow has to start over. */
  onSessionExpired: () => void;
}

/** Sets up the picked code factors, one at a time. */
export const ConfigureFactorStep = ({ onCancel, onSessionExpired }: Props) => {
  const sessionId = useConfigureMfaStore((s) => s.sessionId);
  const instanceName = useConfigureMfaStore((s) => s.instance?.name);
  const method = useConfigureMfaStore(
    selectPendingMethod(ConfigureMfaStep.Configuration),
  );

  const [totpSecret, setTotpSecret] = useState<string | null>(null);
  const [code, setCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleApiError = useMfaConfigErrorHandler({
    context: `MFA configuration of ${method} failed`,
    setError,
    onSessionExpired,
    fallback: 'Configuration failed',
  });

  const { mutate: startSetup, isPending: isStartingSetup } = useMutation({
    mutationFn: async () => {
      if (!isPresent(sessionId) || !isPresent(method)) {
        throw new Error('No MFA configuration session');
      }
      return api.mfaConfigSetupStart(sessionId, method);
    },
    onError: handleApiError,
    onSuccess: (result) => {
      setTotpSecret(result.totp_secret);
    },
  });

  // For email a second call invalidates the code sent, so fire once, StrictMode included.
  const startedFor = useRef<string | null>(null);
  useEffect(() => {
    if (!isPresent(sessionId) || !isPresent(method)) return;
    const key = `${sessionId}:${method}`;
    if (startedFor.current === key) return;
    startedFor.current = key;
    // The step stays mounted between two factors, so the previous one's input has to go.
    setTotpSecret(null);
    setCode(null);
    setError(null);
    startSetup();
  }, [sessionId, method, startSetup]);

  const { mutate: finishSetup, isPending: isFinishingSetup } = useMutation({
    mutationFn: async (value: string) => {
      if (!isPresent(sessionId) || !isPresent(method)) {
        throw new Error('No MFA configuration session');
      }
      const result = await api.mfaConfigSetupFinish(sessionId, method, value);
      return { method, result };
    },
    onError: handleApiError,
    onSuccess: ({ method: configured, result }) => {
      const store = useConfigureMfaStore.getState();
      store.factorConfigured(configured, result.recovery_codes);
      store.next();
    },
  });

  const { mutate: cancel, isPending: isCancelling } = useMutation({
    mutationFn: discardMfaConfiguration,
    onSettled: onCancel,
  });

  const isBusy = isStartingSetup || isFinishingSetup || isCancelling;

  const handleSubmit = useCallback(
    (pastedCode?: string) => {
      if (isBusy) return;
      const toSubmit = (pastedCode ?? code)?.trim();
      if (toSubmit?.length !== CODE_LENGTH) {
        setError('Enter a valid code');
        return;
      }
      finishSetup(toSubmit);
    },
    [code, finishSetup, isBusy],
  );

  // Only real input clears the error, CodeInput's own reset passes ''.
  const handleCodeChange = useCallback((value: string) => {
    setCode(value);
    if (value.length > 0) setError(null);
  }, []);

  // Configuring the last factor empties the list before the wizard swaps this step out.
  if (!isPresent(method)) return null;

  const isTotp = method === MfaMethod.Totp;

  return (
    <div id="configure-factor-step" className="step-content">
      <header>
        <h1>Configure MFA</h1>
        <p>
          {isTotp
            ? `Scan this QR code using an authenticator app (Google Auth, Microsoft Auth etc)`
            : `We've sent a verification code to your email address.`}
        </p>
      </header>
      {isTotp && <TotpSetup secret={totpSecret} instanceName={instanceName} />}
      {!isTotp && <SizedBox height={ThemeSpacing.Xl2} />}
      <p className="code-label">
        {isTotp
          ? `Enter 6-digit code from authentication app`
          : `Enter 6-digit code from email`}
      </p>
      <div className="code-track">
        <CodeInput
          length={CODE_LENGTH}
          value={code}
          onChange={handleCodeChange}
          error={error}
          onSubmit={() => {
            handleSubmit();
          }}
          onSuccessPaste={(value) => {
            handleSubmit(value);
          }}
        />
      </div>
      <Controls>
        <Button
          text="Cancel"
          variant={ButtonVariant.Secondary}
          loading={isCancelling}
          onClick={() => {
            cancel();
          }}
        />
        <div className="right">
          <Button
            text="Configure"
            variant={ButtonVariant.Primary}
            loading={isStartingSetup || isFinishingSetup}
            disabled={isCancelling}
            onClick={() => {
              handleSubmit();
            }}
          />
        </div>
      </Controls>
    </div>
  );
};

type TotpSetupProps = {
  secret: string | null;
  instanceName?: string;
};

const TotpSetup = ({ secret, instanceName }: TotpSetupProps) => {
  // Without an issuer every Defguard account is the same unlabeled row in the authenticator.
  const qrData = useMemo(() => {
    const label = encodeURIComponent(`Defguard:${instanceName ?? 'account'}`);
    return `otpauth://totp/${label}?secret=${secret}&issuer=Defguard`;
  }, [secret, instanceName]);

  return (
    <Fragment>
      <div className="qr-track">
        {isPresent(secret) && <QrCard value={qrData} size={184} />}
      </div>
      <p className="scan-hint">{`Can't scan QR code?  Enter code manually in the app.`}</p>
      {isPresent(secret) && (
        <CopyField copyTooltip="Code copied to clipboard" text={secret} />
      )}
      <Divider spacing={ThemeSpacing.Xl2} />
    </Fragment>
  );
};
