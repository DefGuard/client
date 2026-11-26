import './style.scss';
import { getRouteApi, useNavigate } from '@tanstack/react-router';
import { writeText } from '@tauri-apps/plugin-clipboard-manager';
import { error as logError } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { useCallback } from 'react';
import { Button } from '../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../shared/components/Button/types';
import { Icon, IconKind } from '../../../shared/components/Icon';
import { SizedBox } from '../../../shared/components/SizedBox/SizedBox';
import { FullPage } from '../../../shared/layouts/FullPage/FullPage';
import { useAppData } from '../../../shared/providers/AppDataContext';
import { Snackbar } from '../../../shared/providers/snackbar/snackbar';
import { ThemeSpacing, ThemeVariable } from '../../../shared/types';
import { formatDuration } from '../../../shared/utils/formatDuration';
import { VerificationCode } from './components/VerificationCode/VerificationCode';
import { deviceVerificationErrorMessage } from './errorMessage';
import { useDeviceVerification } from './hooks/useDeviceVerification';
import { DEVICE_BLOCKED_MESSAGE } from './startVerification';

const routeApi = getRouteApi('/full/device-verification');

export const DeviceVerificationPage = () => {
  const { instanceId } = routeApi.useSearch();
  const start = routeApi.useLoaderData();
  const navigate = useNavigate();
  const { setViewSelection } = useAppData();

  // The route's onLeave stops waiting for the decision.
  const leave = useCallback(async () => {
    setViewSelection({ kind: 'instance', id: instanceId });
    await navigate({ to: '/full/overview', replace: true });
  }, [instanceId, navigate, setViewSelection]);

  const { status, code, secondsLeft } = useDeviceVerification({
    instanceId,
    start,
    onVerified: () => {
      void leave().then(() => Snackbar.success('Device successfully verified'));
    },
    onBlocked: () => {
      void leave().then(() => Snackbar.error(DEVICE_BLOCKED_MESSAGE));
    },
  });

  const handleCopy = useCallback(() => {
    if (!code) return;
    writeText(code)
      .then(() => Snackbar.default('Code copied to clipboard'))
      .catch((e) => logError(`Failed to copy device verification code: ${String(e)}`));
  }, [code]);

  const closeButton = (
    <Button
      text="Close"
      variant={ButtonVariant.Secondary}
      onClick={() => {
        void leave();
      }}
    />
  );

  if (status.kind === 'stopped' || status.kind === 'expired') {
    return (
      <FullPage id="device-verification-page" hideScrollContainer>
        <Icon
          icon={IconKind.CheckCircle}
          size={28}
          staticColor={ThemeVariable.FgWhite50}
        />
        <SizedBox height={ThemeSpacing.Xl2} />
        <p className="title">
          {status.kind === 'stopped' ? 'Verification stopped' : 'Verification expired'}
        </p>
        <SizedBox height={ThemeSpacing.Sm} />
        <p className="subtitle">
          {status.kind === 'stopped'
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
      {status.kind === 'error' ? (
        <>
          <p className="error">{deviceVerificationErrorMessage(status.error)}</p>
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="actions">{closeButton}</div>
        </>
      ) : (
        <>
          {code && <VerificationCode code={code} />}
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="divider" />
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="refresh">
            <Icon icon={IconKind.Time} size={16} staticColor={ThemeVariable.FgWhite60} />
            <p>
              Code refreshes in:{' '}
              <span>
                {secondsLeft !== null
                  ? formatDuration(dayjs.duration(secondsLeft, 'seconds'))
                  : '–'}
              </span>
            </p>
          </div>
          <SizedBox height={ThemeSpacing.Xl2} />
          <div className="actions">
            <Button
              text="Copy code to clipboard"
              variant={ButtonVariant.Outlined}
              disabled={!code}
              onClick={handleCopy}
            />
            {closeButton}
          </div>
        </>
      )}
    </FullPage>
  );
};
