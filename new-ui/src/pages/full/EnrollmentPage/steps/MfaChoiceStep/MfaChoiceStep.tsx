/** biome-ignore-all lint/style/noNonNullAssertion: this cannot be shown without those store values */
import { useMutation } from '@tanstack/react-query';
import clsx from 'clsx';
import { useState } from 'react';
import { useShallow } from 'zustand/shallow';
import {
  Icon,
  IconKind,
  type IconKindValue,
} from '../../../../../shared/components/Icon';
import { RadioIndicator } from '../../../../../shared/components/RadioIndicator/RadioIndicator';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { edgeApi } from '../../../../../shared/edge-api/api';
import { MfaMethod, type MfaMethodValue } from '../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../shared/types';
import { EnrollmentControls } from '../../components/EnrollmentControls/EnrollmentControls';
import { useEnrollmentStore } from '../../hooks/useEnrollmentStore';

export const MfaChoiceStep = () => {
  const [proxyUrl, cookie] = useEnrollmentStore(
    useShallow((s) => [s.proxyUrl!, s.sessionCookie!]),
  );
  const [mfa, setMfa] = useState<MfaMethodValue>(
    useEnrollmentStore.getState().userMfaChoice,
  );

  const { mutate, isPending } = useMutation({
    mutationFn: () => edgeApi.startMfaSetup(proxyUrl, cookie, mfa),
    onSuccess: (resp) => {
      if (mfa === MfaMethod.Totp) {
        const secret = resp.result?.totp_secret;
        useEnrollmentStore.setState({
          userMfaChoice: mfa,
          userTotpSecret: secret,
        });
      } else {
        useEnrollmentStore.setState({ userMfaChoice: mfa });
      }
      useEnrollmentStore.getState().next();
    },
  });

  return (
    <div id="mfa-choice-step">
      <header>
        <h1>Enable Multi-Factor Authentication</h1>
        <p>
          To keep your account safe, you need to set up at least one additional sign-in
          step. Use this wizard to get started.
        </p>
      </header>
      <SizedBox height={ThemeSpacing.Xl3} />
      <div className="methods">
        <MethodSelectionCard
          active={mfa === MfaMethod.Totp}
          onChange={() => {
            setMfa(MfaMethod.Totp);
          }}
          icon={IconKind.MobileLock}
          title="Security code from your authenticator app"
          description="Use an app on your phone that generates a temporary code to confirm it's you (Google Authenticator, Microsoft Authenticator, Authy)."
        />
        <MethodSelectionCard
          active={mfa === MfaMethod.Email}
          onChange={() => {
            setMfa(MfaMethod.Email);
          }}
          icon={IconKind.Mail}
          title="Code from email"
          description="We'll send a temporary security code to your email. Enter the code to confirm it's you and continue."
        />
      </div>
      <EnrollmentControls
        disableBack
        onNext={() => {
          mutate();
        }}
        loading={isPending}
      />
    </div>
  );
};

const MethodSelectionCard = ({
  active,
  title,
  description,
  icon,
  onChange,
}: {
  icon: IconKindValue;
  active: boolean;
  title: string;
  description: string;
  onChange: () => void;
}) => {
  return (
    <div
      className={clsx('method-selection-block', {
        active,
      })}
      onClick={onChange}
    >
      <div className="track">
        <div className="icon-track">
          <Icon icon={icon} size={20} />
        </div>
        <div className="info-block">
          <p className="title">{title}</p>
          <p className="description">{description}</p>
        </div>
        <div className="indicator-track">
          <RadioIndicator active={active} />
        </div>
      </div>
    </div>
  );
};
