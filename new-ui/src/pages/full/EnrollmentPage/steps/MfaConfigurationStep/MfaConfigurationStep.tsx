import { Fragment, useCallback, useEffect, useMemo, useState } from 'react';
import './style.scss';
import { CodeInput } from '../../../../../shared/components/CodeInput/CodeInput';
import { CopyField } from '../../../../../shared/components/CopyField/CopyField';
import { Divider } from '../../../../../shared/components/Divider/Divider';
import { QrCard } from '../../../../../shared/components/QrCard/QrCard';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { MfaMethod } from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';
import { EnrollmentControls } from '../../components/EnrollmentControls/EnrollmentControls';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const MfaConfigurationStep = () => {
  const method = useEnrollmentStore((s) => s.userMfaChoice);
  const [code, setCode] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const handleSubmit = useCallback(() => {
    if (code?.length !== 6) {
      setError('');
    }
  }, [code]);

  // biome-ignore lint/correctness/useExhaustiveDependencies: side effect of code
  useEffect(() => {
    setError(null);
  }, [code]);

  return (
    <div id="mfa-configuration-step" className="step-content">
      <header>
        <h1>Configure MFA</h1>
        <p>
          {`Scan this QR code using an authenticator app (Google Auth, Microsoft Auth etc)`}
        </p>
      </header>
      {method === MfaMethod.Email && <SizedBox height={ThemeSpacing.Xl2} />}
      {method === MfaMethod.Totp && <TotpSetup />}
      {method === MfaMethod.Totp && (
        <p className="code-label">{`Enter 6-digit code from authentication app`}</p>
      )}
      {method === MfaMethod.Email && (
        <p className="code-label">{`Enter 6-digit code from email`}</p>
      )}
      <CodeInput
        length={6}
        value={code}
        onChange={setCode}
        onSuccessPaste={() => {
          handleSubmit();
        }}
        error={error}
      />
      <EnrollmentControls onNext={handleSubmit} />
    </div>
  );
};

export const TotpSetup = () => {
  const secret = useEnrollmentStore((s) => s.userTotpSecret);

  const qrData = useMemo(() => {
    if (!secret) return 'Test secret';
    return `otpauth://totp/Defguard?secret=${secret}`;
  }, [secret]);

  return (
    <Fragment>
      <div className="qr-track">
        {isPresent(qrData) && <QrCard value={qrData} size={184} />}
      </div>
      <p className="scan-hint">
        {`Can't scan QR code?  Enter code manually in the app.`}
      </p>
      <CopyField copyTooltip="Code copied to clipboard" text="Test secret" />
      <Divider spacing={ThemeSpacing.Xl2} />
    </Fragment>
  );
};
