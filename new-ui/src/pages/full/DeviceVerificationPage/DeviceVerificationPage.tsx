import './style.scss';
import { getRouteApi, useNavigate, useRouter } from '@tanstack/react-router';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { error as logError } from '@tauri-apps/plugin-log';
import { type PropsWithChildren, useState } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Divider } from '../../../shared/components/Divider/Divider';
import { Icon, IconKind, type IconKindValue } from '../../../shared/components/Icon';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { TooltipButton } from '../../../shared/components/TooltipButton/TooltipButton';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useAppData } from '../../../shared/providers/AppDataContext';
import { Snackbar } from '../../../shared/providers/snackbar/snackbar';
import { ThemeSpacing, ThemeVariable } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { VerificationCode } from './components/VerificationCode/VerificationCode';
import { useDeviceVerification } from './hooks/useDeviceVerification';
import type { DeviceVerificationSession } from './types';
import { deviceVerificationErrorMessage } from './utils/deviceVerificationErrorMessage';
import { openDeviceBlockedModal } from './utils/openDeviceBlockedModal';

const routeApi = getRouteApi('/full/device-verification');

const formatSeconds = (seconds: number) => `${seconds} sec`;

export const DeviceVerificationPage = () => {
  const start = routeApi.useLoaderData();
  // a restart reloads the route, remounting resets the outcome of the previous session
  return <DeviceVerificationContent key={start.fetchedAt} start={start} />;
};

type ContentProps = {
  start: DeviceVerificationSession;
};

const DeviceVerificationContent = ({ start }: ContentProps) => {
  const { instanceId } = routeApi.useSearch();
  const navigate = useNavigate();
  const router = useRouter();
  const { setViewSelection } = useAppData();
  const [isRestarting, setRestarting] = useState(false);

  // The route's onLeave stops waiting for the decision.
  const leave = async () => {
    setViewSelection({ kind: 'instance', id: instanceId });
    await navigate({ to: '/full/overview', replace: true });
  };

  const { status, code, secondsLeft } = useDeviceVerification({
    instanceId,
    start,
    onVerified: () => {
      void leave().then(() => Snackbar.success('Device successfully verified'));
    },
    onBlocked: () => {
      void leave().then(openDeviceBlockedModal);
    },
  });

  const handleCopy = () => {
    if (!isPresent(code)) return;
    writeText(code).catch((e) =>
      logError(`Failed to copy device verification code: ${String(e)}`),
    );
  };

  // a failed start redirects to the overview from the loader
  const handleRestart = () => {
    setRestarting(true);
    void router
      .invalidate({ filter: (match) => match.routeId === '/full/device-verification' })
      .finally(() => {
        setRestarting(false);
      });
  };

  const closeButton = (
    <Button
      text="Close"
      variant={ButtonVariant.Secondary}
      onClick={() => {
        void leave();
      }}
    />
  );

  switch (status.kind) {
    case 'cancelled':
      return (
        <Outcome
          icon={IconKind.CheckCircle}
          title="Verification stopped"
          subtitle={[
            'The verification process was stopped by your administrator.',
            'You can close this window.',
          ]}
        >
          {closeButton}
        </Outcome>
      );
    case 'expired':
      return (
        <Outcome
          icon={IconKind.Expired}
          title="Verification session expired."
          subtitle={[
            'The verification session has ended.',
            'You can restart verification now or do it later.',
          ]}
        >
          <Button
            text="Restart verification"
            variant={ButtonVariant.Outlined}
            iconLeft={IconKind.Refresh}
            loading={isRestarting}
            onClick={handleRestart}
          />
          {closeButton}
        </Outcome>
      );
    case 'error':
      return (
        <Outcome
          icon={IconKind.Denied}
          title="Verification failed."
          subtitle={[deviceVerificationErrorMessage(status.error)]}
        >
          {closeButton}
        </Outcome>
      );
    case 'pending':
      return (
        <FullPage id="device-verification-page" hideScrollContainer>
          <Icon
            icon={IconKind.NotVerified}
            size={28}
            staticColor={ThemeVariable.FgWhite50}
          />
          <SizedBox height={ThemeSpacing.Xl2} />
          <p className="title">Device verification</p>
          <SizedBox height={ThemeSpacing.Sm} />
          <p className="subtitle">
            Your administrator may ask you to verify this device. When requested, read the
            code below or send it to your administrator.
          </p>
          <SizedBox height={ThemeSpacing.Xl2} />
          {isPresent(code) && <VerificationCode code={code} />}
          <SizedBox height={ThemeSpacing.Xl2} />
          <Divider />
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="refresh">
            <div className="label">
              <Icon
                icon={IconKind.Time}
                size={16}
                staticColor={ThemeVariable.FgWhite60}
              />
              <p>Code refreshes in:</p>
            </div>
            <div className="timer">
              <p className="value">
                {isPresent(secondsLeft) ? formatSeconds(secondsLeft) : '–'}
              </p>
              <p className="placeholder" aria-hidden>
                {formatSeconds(start.info.period)}
              </p>
            </div>
          </div>
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="actions">
            <TooltipButton
              tooltipText="Code copied to clipboard"
              buttonProps={{
                text: 'Copy code to clipboard',
                variant: ButtonVariant.Outlined,
                iconLeft: IconKind.Copy,
                disabled: !isPresent(code),
                onClick: handleCopy,
              }}
            />
            {closeButton}
          </div>
        </FullPage>
      );
  }
};

type OutcomeProps = PropsWithChildren<{
  icon: IconKindValue;
  title: string;
  subtitle: string[];
}>;

const Outcome = ({ icon, title, subtitle, children }: OutcomeProps) => {
  return (
    <FullPage id="device-verification-page" className="outcome" hideScrollContainer>
      <Icon icon={icon} size={28} staticColor={ThemeVariable.FgWhite50} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <p className="title">{title}</p>
      <SizedBox height={ThemeSpacing.Sm} />
      <p className="subtitle">
        {subtitle.map((line, index) => (
          <span key={index}>{line}</span>
        ))}
      </p>
      <SizedBox height={ThemeSpacing.Xl2} />
      <div className="actions">{children}</div>
    </FullPage>
  );
};
