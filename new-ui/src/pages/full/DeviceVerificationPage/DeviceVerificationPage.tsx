import './style.scss';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { error as logError } from '@tauri-apps/plugin-log';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Divider } from '../../../shared/components/Divider/Divider';
import { Icon, IconKind } from '../../../shared/components/Icon';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { TooltipButton } from '../../../shared/components/TooltipButton/TooltipButton';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useAppData } from '../../../shared/providers/AppDataContext';
import { Snackbar } from '../../../shared/providers/snackbar/snackbar';
import { ThemeSpacing, ThemeVariable } from '../../../shared/types';
import { isPresent } from '../../../shared/utils/isPresent';
import { VerificationCode } from './components/VerificationCode/VerificationCode';
import { useDeviceVerification } from './hooks/useDeviceVerification';
import {
  deviceBlockedMessage,
  deviceVerificationErrorMessage,
} from './utils/deviceVerificationErrorMessage';

const routeApi = getRouteApi('/full/device-verification');

const formatSeconds = (seconds: number) => `${seconds} seconds`;

export const DeviceVerificationPage = () => {
  const { instanceId } = routeApi.useSearch();
  const start = routeApi.useLoaderData();
  const navigate = useNavigate();
  const { setViewSelection } = useAppData();

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
      void leave().then(() => Snackbar.error(deviceBlockedMessage));
    },
  });

  const handleCopy = () => {
    if (!isPresent(code)) return;
    writeText(code).catch((e) =>
      logError(`Failed to copy device verification code: ${String(e)}`),
    );
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

  if (status.kind === 'cancelled' || status.kind === 'expired') {
    return (
      <FullPage id="device-verification-page" hideScrollContainer>
        <Icon
          icon={IconKind.CheckCircle}
          size={28}
          staticColor={ThemeVariable.FgWhite50}
        />
        <SizedBox height={ThemeSpacing.Xl2} />
        <p className="title">
          {status.kind === 'cancelled' ? 'Verification stopped' : 'Verification expired'}
        </p>
        <SizedBox height={ThemeSpacing.Sm} />
        <p className="subtitle">
          {status.kind === 'cancelled'
            ? 'The verification process was stopped by your administrator. You can close this window.'
            : 'The verification time limit has passed. Start the verification again from the instance settings when your administrator asks for it.'}
        </p>
        <SizedBox height={ThemeSpacing.Xl2} />
        <div className="actions">{closeButton}</div>
      </FullPage>
    );
  }

  return (
    <FullPage id="device-verification-page" hideScrollContainer>
      <Icon icon={IconKind.NotVerified} size={28} staticColor={ThemeVariable.FgWhite50} />
      <SizedBox height={ThemeSpacing.Xl2} />
      <p className="title">Device verification</p>
      <SizedBox height={ThemeSpacing.Sm} />
      <p className="subtitle">
        Your administrator may ask you to verify this device. When requested, read the
        code below or send it to your administrator.
      </p>
      <SizedBox height={ThemeSpacing.Xl2} />
      {status.kind === 'error' && (
        <>
          <p className="error">{deviceVerificationErrorMessage(status.error)}</p>
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="actions">{closeButton}</div>
        </>
      )}
      {status.kind === 'pending' && (
        <>
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
              {start.ok && (
                <p className="placeholder" aria-hidden>
                  {formatSeconds(start.info.period)}
                </p>
              )}
            </div>
          </div>
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="actions">
            <TooltipButton
              tooltipText="Code copied to clipboard"
              buttonProps={{
                text: 'Copy code to clipboard',
                variant: ButtonVariant.Outlined,
                disabled: !isPresent(code),
                onClick: handleCopy,
              }}
            />
            {closeButton}
          </div>
        </>
      )}
    </FullPage>
  );
};
