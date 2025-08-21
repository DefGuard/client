import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { isUndefined } from 'lodash-es';
import { useCallback, useEffect, useMemo, useState } from 'react';
import AuthCode from 'react-auth-code-input';
import { type SubmitHandler, useForm } from 'react-hook-form';
import ReactMarkdown from 'react-markdown';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { LoaderSpinner } from '../../../../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { MessageBox } from '../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useToaster } from '../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { isPresent } from '../../../../../../../../shared/defguard-ui/utils/isPresent';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import {
  type DefguardInstance,
  LocationMfaType,
} from '../../../../../../types';
import { MfaMobileApprove } from './components/MfaMobileApprove/MfaMobileApprove';
import { BrowserErrorIcon, BrowserPendingIcon, GoToBrowserIcon } from './Icons';
import { useMFAModal } from './useMFAModal';

const { connect } = clientApi;

const CODE_LENGTH = 6;
const CLIENT_MFA_ENDPOINT = 'api/v1/client-mfa';

type FormFields = {
  code: string;
};

type MFAError = {
  error: string;
};

const defaultValues: FormFields = {
  code: '',
};

type MFAStartResponse = {
  token: string;
  challenge?: string;
};

type Screen =
  | 'start'
  | 'authenticator_app'
  | 'email'
  | 'openid_login'
  | 'openid_pending'
  | 'openid_unavailable'
  | 'mobile_approve';

export const MFAModal = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();

  const [authMethod, setAuthMethod] = useState<number>(0);
  const [screen, setScreen] = useState<Screen>('start');
  const [proxyUrl, setProxyUrl] = useState('');
  const [startResponse, setStartResponse] = useState<MFAStartResponse>();

  const localLL = LL.modals.mfa.authentication;
  const [isOpen, location] = useMFAModal((state) => [state.isOpen, state.instance]);
  const [close, reset] = useMFAModal((state) => [state.close, state.reset], shallow);
  const instances = useClientStore((state) => state.instances);
  const selectedInstance = useMemo((): DefguardInstance | undefined => {
    const instanceId = location?.instance_id;
    if (!isUndefined(instanceId)) {
      return instances.find((i) => i.id === instanceId);
    }
  }, [location, instances]);

  const resetState = () => {
    reset();
    setScreen('start');
    setStartResponse(undefined);
  };

  const resetAuthState = () => {
    setScreen('start');
    setStartResponse(undefined);
  };

  // selectedMethod: 0 = authenticator app, 1 = email, 2 = OpenID, 3 = MobileApprove
  const startMFA = useCallback(
    async (method: number) => {
      if (!location) return toaster.error(localLL.errors.locationNotSpecified());

      if (!selectedInstance) {
        return toaster.error(localLL.errors.instanceNotFound());
      }

      setProxyUrl(selectedInstance.proxy_url);
      const mfaStartUrl = `${selectedInstance.proxy_url + CLIENT_MFA_ENDPOINT}/start`;

      const data = {
        method,
        pubkey: selectedInstance.pubkey,
        location_id: location.network_id,
      };

      const response = await fetch(mfaStartUrl, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(data),
      });

      if (response.ok) {
        const data = (await response.json()) as MFAStartResponse;

        switch (method) {
          case 0:
            setScreen('authenticator_app');
            break;
          case 1:
            setScreen('email');
            break;
          case 2:
            setScreen('openid_login');
            break;
          case 4:
            // just to be safe
            if (!isPresent(data.challenge)) {
              toaster.error('Unsupported response from proxy');
            }
            setScreen('mobile_approve');
            break;
          default:
            toaster.error(localLL.errors.mfaStartGeneric());
            return;
        }
        setStartResponse(data);
        return data;
      } else {
        const errorData = ((await response.json()) as unknown as MFAError).error;
        error(`MFA failed to start with the following error: ${errorData}`);
        if (method === 2) {
          setScreen('openid_unavailable');
          return;
        }

        if (errorData === 'selected MFA method is not available') {
          toaster.error(localLL.errors.mfaNotConfigured());
        } else {
          toaster.error(localLL.errors.mfaStartGeneric());
        }

        return;
      }
    },
    [
      localLL.errors.instanceNotFound,
      localLL.errors.locationNotSpecified,
      localLL.errors.mfaNotConfigured,
      localLL.errors.mfaStartGeneric,
      location,
      selectedInstance,
      toaster.error,
    ],
  );

  const useOpenIDMFA = useMemo(() => {
    return location?.location_mfa_mode === LocationMfaType.EXTERNAL;
  }, [location]);

  const { mutate, isPending } = useMutation({
    mutationFn: startMFA,
  });

  const handleMfaStart = (method: number) => {
    setAuthMethod(method);
    mutate(method);
  };

  return (
    <ModalWithTitle
      id="mfa-modal"
      title={localLL.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={resetState}
    >
      {useOpenIDMFA && screen === 'start' && (
        <OpenIDMFAStart
          isPending={isPending}
          showOpenIDScreen={() => {
            handleMfaStart(2);
          }}
        />
      )}
      {useOpenIDMFA && screen === 'openid_unavailable' && (
        <OpenIDMFAUnavailable resetState={resetAuthState} />
      )}
      {screen === 'start' && !useOpenIDMFA && (
        <MFAStart
          isPending={isPending}
          authMethod={authMethod}
          startMfa={handleMfaStart}
        />
      )}
      {screen === 'openid_login' && isPresent(startResponse) && (
        <OpenIDMFALogin
          proxyUrl={proxyUrl}
          token={startResponse?.token}
          resetAuthState={resetAuthState}
          setScreen={setScreen}
          openidDisplayName={selectedInstance?.openid_display_name}
        />
      )}
      {screen === 'openid_pending' && isPresent(startResponse) && (
        <OpenIDMFAPending
          proxyUrl={proxyUrl}
          token={startResponse.token}
          resetState={resetAuthState}
        />
      )}
      {(screen === 'authenticator_app' || screen === 'email') &&
        isPresent(startResponse) && (
          <MFACodeForm
            description={
              screen === 'authenticator_app'
                ? localLL.authenticatorAppDescription()
                : localLL.emailCodeDescription()
            }
            token={startResponse.token}
            proxyUrl={proxyUrl}
            resetState={resetAuthState}
          />
        )}
      {screen === 'mobile_approve' &&
        isPresent(startResponse) &&
        isPresent(selectedInstance) && (
          <MfaMobileApprove
            proxyUrl={proxyUrl}
            instanceUuid={selectedInstance.uuid}
            onCancel={resetAuthState}
            data={{
              challenge: startResponse.challenge as string,
              token: startResponse.token,
            }}
          />
        )}
    </ModalWithTitle>
  );
};

type MFAStartProps = {
  isPending: boolean;
  authMethod: number;
  startMfa: (method: number) => void;
};

const OpenIDMFAUnavailable = ({ resetState }: { resetState: () => void }) => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.mfa.authentication;

  return (
    <div className="mfa-modal-content">
      <div className="mfa-modal-content-icon">
        <BrowserErrorIcon />
      </div>
      <div className="mfa-modal-content-description mfa-model-error-description">
        <p>{localLL.openidUnavailable.description()}</p>
      </div>
      <div className="mfa-modal-content-footer">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          text={localLL.openidUnavailable.tryAgain()}
          onClick={resetState}
        />
      </div>
    </div>
  );
};

const OpenIDMFAStart = ({
  isPending,
  showOpenIDScreen,
}: {
  isPending: boolean;
  showOpenIDScreen: () => void;
}) => {
  useEffect(() => {
    if (!isPending) {
      showOpenIDScreen();
    }
  }, [isPending, showOpenIDScreen]);

  return (
    <div className="mfa-modal-content">
      <LoaderSpinner size={50} />
    </div>
  );
};

const MFAStart = ({ isPending, authMethod, startMfa }: MFAStartProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.mfa.authentication;

  const isAuthenticatorAppPending = isPending && authMethod === 0;
  const isEmailCodePending = isPending && authMethod === 1;

  return (
    <div className="mfa-modal-content">
      <div className="mfa-modal-content-description">
        <p>{localLL.mfaStartDescriptionPrimary()}</p>
        <p>{localLL.mfaStartDescriptionSecondary()}</p>
      </div>

      <div className="mfa-modal-content-button-container">
        <Button
          disabled={isPending}
          size={ButtonSize.LARGE}
          loading={isAuthenticatorAppPending}
          styleVariant={ButtonStyleVariant.STANDARD}
          // biome-ignore lint/correctness/useHookAtTopLevel: not a hook
          text={isAuthenticatorAppPending ? '' : localLL.useAuthenticatorApp()}
          onClick={() => {
            startMfa(0);
          }}
        />
        <Button
          disabled={isPending}
          size={ButtonSize.LARGE}
          loading={isEmailCodePending}
          styleVariant={ButtonStyleVariant.STANDARD}
          // biome-ignore lint/correctness/useHookAtTopLevel: it's not hook
          text={isEmailCodePending ? '' : localLL.useEmailCode()}
          onClick={() => {
            startMfa(1);
          }}
        />
        <Button
          disabled={isPending}
          size={ButtonSize.LARGE}
          loading={isEmailCodePending}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={'Use Mobile Client'}
          onClick={() => {
            startMfa(4);
          }}
        />
      </div>
    </div>
  );
};

type MFACodeForm = {
  description: string;
  token: string;
  proxyUrl: string;
  resetState: () => void;
};

type MFAFinishResponse = {
  preshared_key: string;
};

const OpenIDMFALogin = ({
  proxyUrl,
  token,
  setScreen,
  openidDisplayName,
}: {
  proxyUrl: string;
  token: string;
  openidDisplayName?: string;
  resetAuthState: () => void;
  setScreen: (screen: Screen) => void;
}) => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.mfa.authentication;
  const { openLink } = clientApi;
  const displayName = openidDisplayName || 'OpenID provider';

  return (
    <div className="mfa-modal-content">
      <div className="mfa-modal-content-icon">
        <GoToBrowserIcon />
      </div>
      <div className="mfa-modal-content-description">
        <p>{localLL.openidLogin.description({ provider: displayName })}</p>
        <br />
        <ReactMarkdown>
          {localLL.openidLogin.browserWarning({ provider: displayName })}
        </ReactMarkdown>
      </div>
      <div className="mfa-modal-content-button-container">
        <Button
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={localLL.openidLogin.buttonText({ provider: displayName })}
          onClick={() => {
            const link = `${proxyUrl}openid/mfa?token=${token}`;
            openLink(link);
            setScreen('openid_pending');
          }}
        />
      </div>
    </div>
  );
};

type OpenIDMFAPendingProps = {
  proxyUrl: string;
  token: string;
  resetState: () => void;
};

const OpenIDMFAPending = ({ proxyUrl, token, resetState }: OpenIDMFAPendingProps) => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.mfa.authentication;
  const toaster = useToaster();
  const location = useMFAModal((state) => state.instance);
  const closeModal = useMFAModal((state) => state.close);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);

  useEffect(() => {
    const TIMEOUT_DURATION = 5 * 1000 * 60; // 5 minutes timeout
    let timeoutId: NodeJS.Timeout;

    const pollMFAStatus = async () => {
      if (!location) {
        toaster.error(localLL.errors.mfaStartGeneric());
        setErrorMessage(localLL.errors.locationNotSpecified());
        return;
      }

      const body_token = { token };
      const response = await fetch(`${proxyUrl + CLIENT_MFA_ENDPOINT}/finish`, {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: JSON.stringify(body_token),
      });

      if (response.ok) {
        clearInterval(interval);
        clearTimeout(timeoutId);
        closeModal();
        const data = (await response.json()) as MFAFinishResponse;
        await connect({
          locationId: location?.id,
          connectionType: location.connection_type,
          presharedKey: data.preshared_key,
        });
        return;
      }

      // HTTP 428: Precondition required, continue waiting, the user may have not completed the OpenID login yet
      if (response.status === 428) {
        return;
      }

      // Other errors: stop polling and handle
      clearInterval(interval);
      clearTimeout(timeoutId);
      const data = (await response.json()) as unknown as MFAError;
      const { error: errorMessage } = data;

      if (errorMessage === 'invalid token') {
        error(JSON.stringify(data, null, 2));
        setErrorMessage(localLL.errors.tokenExpired());
      } else if (errorMessage === 'login session not found') {
        error(JSON.stringify(data, null, 2));
        setErrorMessage(localLL.errors.sessionInvalidated());
      } else {
        error(JSON.stringify(data, null, 2));
        setErrorMessage(localLL.errors.mfaStartGeneric());
      }
    };

    const handleTimeout = () => {
      clearInterval(interval);
      clearTimeout(timeoutId);
      setErrorMessage(localLL.errors.authenticationTimeout());
    };

    const interval = setInterval(pollMFAStatus, 5000);
    timeoutId = setTimeout(handleTimeout, TIMEOUT_DURATION);

    return () => {
      clearInterval(interval);
      clearTimeout(timeoutId);
    };
  }, [proxyUrl, token, location, closeModal, localLL.errors, toaster]);

  return (
    <div className="mfa-modal-content">
      {!errorMessage ? (
        <>
          <div className="mfa-modal-content-icon">
            <div className="icon-spinner">
              <LoaderSpinner size={49} />
            </div>
            <BrowserPendingIcon />
          </div>
          <div className="mfa-modal-content-description">
            <p>{localLL.openidPending.description()}</p>
          </div>
        </>
      ) : (
        <>
          <div className="mfa-modal-content-icon">
            <BrowserErrorIcon />
          </div>
          <div className="mfa-modal-content-description mfa-model-error-description">
            <p>{localLL.openidPending.errorDescription()}</p>
            <p className="mfa-model-error-message">{errorMessage}</p>
          </div>
        </>
      )}
      <div className="mfa-modal-content-footer">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          text={localLL.openidPending.tryAgain()}
          onClick={() => {
            resetState();
          }}
        />
      </div>
    </div>
  );
};

const MFACodeForm = ({ description, token, proxyUrl, resetState }: MFACodeForm) => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const location = useMFAModal((state) => state.instance);
  const closeModal = useMFAModal((state) => state.close);

  const [mfaError, setMFAError] = useState('');

  const localLL = LL.modals.mfa.authentication;

  const schema = useMemo(
    () =>
      z.object({
        code: z.string().trim().length(6),
      }),
    [],
  );

  const finishMFA = async (code: string) => {
    if (!location) return toaster.error(localLL.errors.mfaStartGeneric());

    const data = { token, code: code };

    const response = await fetch(`${proxyUrl + CLIENT_MFA_ENDPOINT}/finish`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: JSON.stringify(data),
    });

    if (response.ok) {
      closeModal();
      const data = (await response.json()) as MFAFinishResponse;
      await connect({
        locationId: location?.id,
        connectionType: location.connection_type,
        presharedKey: data.preshared_key,
      });
    } else {
      const data = (await response.json()) as unknown as MFAError;
      const { error: errorMessage } = data;
      let message = '';

      if (errorMessage === 'Unauthorized') {
        message = localLL.errors.invalidCode();
      } else if (
        errorMessage === 'invalid token' ||
        errorMessage === 'login session not found'
      ) {
        console.error(data);
        toaster.error(localLL.errors.tokenExpired());
        resetState();
        error(JSON.stringify(data));
        return;
      } else {
        toaster.error(localLL.errors.mfaStartGeneric());
      }

      setMFAError(message);
      error(JSON.stringify(data));
      return;
    }
  };

  const { mutate, isPending } = useMutation({
    mutationFn: finishMFA,
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async ({ code }) => {
    mutate(code);
  };

  const { handleSubmit, setValue } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  return (
    <div className="mfa-modal-content">
      <div className="mfa-modal-content-description">
        <p>{description}</p>
      </div>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <AuthCode
          length={CODE_LENGTH}
          allowedCharacters="numeric"
          containerClassName="mfa-code-container"
          inputClassName="mfa-code-single-character-input"
          onChange={(code: string) => setValue('code', code)}
        />

        <div style={{ height: 75 }}>
          {mfaError ? (
            <MessageBox type={MessageBoxType.ERROR} message={mfaError} />
          ) : null}
        </div>

        <div className="mfa-model-content-footer">
          <Button
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={isPending ? '' : localLL.buttonSubmit()}
            type="submit"
            className="submit"
            size={ButtonSize.SMALL}
            style={{ minWidth: 200 }}
            loading={isPending}
          />
        </div>
      </form>
    </div>
  );
};
