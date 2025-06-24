import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { Body, fetch } from '@tauri-apps/api/http';
import { useCallback, useEffect, useMemo, useState } from 'react';
import AuthCode from 'react-auth-code-input';
import ReactMarkdown from 'react-markdown';
import { SubmitHandler, useForm } from 'react-hook-form';
import { error } from 'tauri-plugin-log-api';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { Button } from '../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useToaster } from '../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useMFAModal } from './useMFAModal';
import { LoaderSpinner } from '../../../../../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { isUndefined } from 'lodash-es';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { DefguardInstance, WireguardInstanceType } from '../../../../../../types';
import { BrowserErrorIcon, BrowserPendingIcon, GoToBrowserIcon } from './Icons';

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
};

type Screen = 'start' | 'authenticator_app' | 'email' | 'openid_login' | 'openid_pending';

export const MFAModal = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();

  const [authMethod, setAuthMethod] = useState<0 | 1 | 2>(0);
  const [screen, setScreen] = useState<Screen>('start');
  const [mfaToken, setMFAToken] = useState('');
  const [proxyUrl, setProxyUrl] = useState('');

  const localLL = LL.modals.mfa.authentication;
  const isOpen = useMFAModal((state) => state.isOpen);
  const location = useMFAModal((state) => state.instance);
  const [close, reset] = useMFAModal((state) => [state.close, state.reset], shallow);
  const [selectedInstanceId, selectedInstanceType] = useClientStore((state) => [
    state.selectedInstance?.id,
    state.selectedInstance?.type,
  ]);
  const instances = useClientStore((state) => state.instances);
  const selectedInstance = useMemo((): DefguardInstance | undefined => {
    if (
      !isUndefined(selectedInstanceId) &&
      selectedInstanceType &&
      selectedInstanceType === WireguardInstanceType.DEFGUARD_INSTANCE
    ) {
      return instances.find((i) => i.id === selectedInstanceId);
    }
  }, [selectedInstanceId, selectedInstanceType, instances]);

  const resetState = () => {
    reset();
    setScreen('start');
    setMFAToken('');
  };

  const resetAuthState = () => {
    setScreen('start');
    setMFAToken('');
  };

  // selectedMethod: 0 = authenticator app, 1 = email, 2 = OpenID
  const startMFA = async (selectedMethod: number) => {
    if (!location) return toaster.error(localLL.errors.locationNotSpecified());

    if (!selectedInstance) {
      return toaster.error(localLL.errors.instanceNotFound());
    }

    setProxyUrl(selectedInstance.proxy_url);
    const mfaStartUrl = selectedInstance.proxy_url + CLIENT_MFA_ENDPOINT + '/start';

    const data = {
      method: selectedMethod,
      pubkey: selectedInstance.pubkey,
      location_id: location.network_id,
    };

    const response = await fetch<MFAStartResponse>(mfaStartUrl, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: Body.json(data),
    });

    if (response.ok) {
      const { token } = response.data;

      switch (selectedMethod) {
        case 0:
          setScreen('authenticator_app');
          break;
        case 1:
          setScreen('email');
          break;
        case 2:
          setScreen('openid_login');
          break;
        default:
          toaster.error(localLL.errors.mfaStartGeneric());
          return;
      }
      setMFAToken(token);

      return response.data;
    } else {
      const error = (response.data as unknown as MFAError).error;
      if (error === 'selected MFA method not available') {
        toaster.error(localLL.errors.mfaNotConfigured());
      } else {
        toaster.error(localLL.errors.mfaStartGeneric());
      }

      return;
    }
  };

  const useOpenIDMFA = useMemo(() => {
    return selectedInstance?.use_openid_for_mfa || false;
  }, [selectedInstance]);

  const { mutate, isPending } = useMutation({
    mutationFn: startMFA,
  });

  const showEmailCodeForm = useCallback(() => {
    setAuthMethod(1);
    mutate(1);
  }, [mutate]);

  const showAuthenticatorAppCodeForm = useCallback(() => {
    setAuthMethod(0);
    mutate(0);
  }, [mutate]);

  const showOpenIDScreen = useCallback(() => {
    setAuthMethod(2);
    mutate(2);
  }, [mutate]);

  return (
    <ModalWithTitle
      id="mfa-modal"
      title={localLL.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={resetState}
    >
      {useOpenIDMFA && screen === 'start' && (
        <OpenIDMFAStart isPending={isPending} showOpenIDScreen={showOpenIDScreen} />
      )}
      {screen === 'start' && !useOpenIDMFA && (
        <MFAStart
          isPending={isPending}
          authMethod={authMethod}
          showEmailCodeForm={showEmailCodeForm}
          showAuthenticatorAppCodeForm={showAuthenticatorAppCodeForm}
          showOpenIDScreen={showOpenIDScreen}
        />
      )}
      {screen === 'openid_login' && (
        <OpenIDMFALogin
          proxyUrl={proxyUrl}
          token={mfaToken}
          resetAuthState={resetAuthState}
          setScreen={setScreen}
          openidDisplayName={selectedInstance?.openid_display_name}
        />
      )}
      {screen === 'openid_pending' && (
        <OpenIDMFAPending
          proxyUrl={proxyUrl}
          token={mfaToken}
          resetState={resetAuthState}
        />
      )}
      {(screen === 'authenticator_app' || screen === 'email') && (
        <MFACodeForm
          description={
            screen === 'authenticator_app'
              ? localLL.authenticatorAppDescription()
              : localLL.emailCodeDescription()
          }
          token={mfaToken}
          proxyUrl={proxyUrl}
          resetState={resetAuthState}
        />
      )}
    </ModalWithTitle>
  );
};

type MFAStartProps = {
  isPending: boolean;
  authMethod: number;
  showAuthenticatorAppCodeForm: () => void;
  showEmailCodeForm: () => void;
  showOpenIDScreen: () => void;
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

const MFAStart = ({
  isPending,
  authMethod,
  showAuthenticatorAppCodeForm,
  showEmailCodeForm,
}: MFAStartProps) => {
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
          text={isAuthenticatorAppPending ? '' : localLL.useAuthenticatorApp()}
          onClick={showAuthenticatorAppCodeForm}
        />
        <Button
          disabled={isPending}
          size={ButtonSize.LARGE}
          loading={isEmailCodePending}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={isEmailCodePending ? '' : localLL.useEmailCode()}
          onClick={showEmailCodeForm}
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
          size={ButtonSize.STANDARD}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={localLL.openidLogin.buttonText({ provider: displayName })}
          onClick={() => {
            const link = proxyUrl + 'openid/mfa?token=' + token;
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
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const TIMEOUT_DURATION = 5 * 60 * 1000;
    let timeoutId: NodeJS.Timeout;

    const pollMFAStatus = async () => {
      if (!location) {
        toaster.error(localLL.errors.mfaStartGeneric());
        setError(localLL.errors.locationNotSpecified());
        return;
      }

      const data = { token };
      const response = await fetch<MFAFinishResponse>(
        proxyUrl + CLIENT_MFA_ENDPOINT + '/finish',
        {
          method: 'POST',
          headers: {
            'Content-Type': 'application/json',
          },
          body: Body.json(data),
        },
      );

      if (response.ok) {
        clearInterval(interval);
        clearTimeout(timeoutId);
        closeModal();

        await connect({
          locationId: location?.id,
          connectionType: location.connection_type,
          presharedKey: response.data.preshared_key,
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
      const { error: errorMessage } = response.data as unknown as MFAError;

      if (
        errorMessage === 'invalid token' ||
        errorMessage === 'login session not found'
      ) {
        console.error(response.data);
        toaster.error(localLL.errors.tokenExpired());
        resetState();
      } else {
        console.error('MFA error:', response.data);
        toaster.error(JSON.stringify(response.data, null, 2));
      }
    };

    const handleTimeout = () => {
      clearInterval(interval);
      toaster.error(localLL.errors.authenticationTimeout());
    };

    const interval = setInterval(pollMFAStatus, 5000);
    timeoutId = setTimeout(handleTimeout, TIMEOUT_DURATION);

    return () => {
      clearInterval(interval);
      clearTimeout(timeoutId);
    };
  }, [proxyUrl, token, location, closeModal, resetState, localLL.errors, toaster]);

  return (
    <div className="mfa-modal-content">
      {!error ? (
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
          </div>
        </>
      )}
      <div className="mfa-modal-content-footer">
        <Button
          styleVariant={ButtonStyleVariant.STANDARD}
          text={localLL.openidPending.tryAgain()}
          size={ButtonSize.STANDARD}
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

    const response = await fetch<MFAFinishResponse>(
      proxyUrl + CLIENT_MFA_ENDPOINT + '/finish',
      {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
        },
        body: Body.json(data),
      },
    );

    if (response.ok) {
      closeModal();

      await connect({
        locationId: location?.id,
        connectionType: location.connection_type,
        presharedKey: response.data.preshared_key,
      });
    } else {
      const { error: errorMessage } = response.data as unknown as MFAError;
      let message = '';

      if (errorMessage === 'Unauthorized') {
        message = localLL.errors.invalidCode();
      } else if (
        errorMessage === 'invalid token' ||
        errorMessage === 'login session not found'
      ) {
        console.error(response.data);
        toaster.error(localLL.errors.tokenExpired());
        resetState();
        error(JSON.stringify(response.data));
        return;
      } else {
        toaster.error(localLL.errors.mfaStartGeneric());
      }

      setMFAError(message);
      error(JSON.stringify(response.data));
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
