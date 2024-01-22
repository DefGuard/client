import "./style.scss";

import { useI18nContext } from "../../../../../../../../i18n/i18n-react";
import { MessageBox } from "../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox";
import { MessageBoxType } from "../../../../../../../../shared/defguard-ui/components/Layout/MessageBox/types";
import { ModalWithTitle } from "../../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle";
import { useMFAModal } from "./useMFAModal";
import { shallow } from "zustand/shallow";
import { Button } from "../../../../../../../../shared/defguard-ui/components/Layout/Button/Button";
import { ButtonSize, ButtonStyleVariant } from "../../../../../../../../shared/defguard-ui/components/Layout/Button/types";
import { useCallback, useMemo, useState } from "react";
import { z } from "zod";
import { SubmitHandler, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { clientApi } from "../../../../../../clientAPI/clientApi";
import { useMutation } from "@tanstack/react-query";
import { useToaster } from "../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster";
import { Body, fetch } from "@tauri-apps/api/http";
import AuthCode from 'react-auth-code-input';

const { connect } = clientApi;

const CODE_LENGTH = 6;
const CLIENT_MFA_ENDPOINT = "api/v1/client-mfa";

type FormFields = {
  code: string;
};

const defaultValues: FormFields = {
  code: '',
};

type MFAStartResponse = {
  token: string;
};

export const MFAModal = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();

  const [authMethod, setAuthMethod] = useState<0 | 1>(0);
  const [screen, setScreen] = useState<'start' | 'authenticator_app' | 'email'>('start');
  const [mfaToken, setMFAToken] = useState('');
  const [proxyUrl, setProxyUrl] = useState('');

  const localLL = LL.modals.mfa.authentication;
  const isOpen = useMFAModal((state) => state.isOpen);
  const location = useMFAModal((state) => state.instance);
  const [close, reset] = useMFAModal(
    (state) => [state.close, state.reset],
    shallow,
  );

  const resetState = () => {
    reset();
    setScreen('start');
    setMFAToken('');
  };

  const resetAuthState = () => {
    setScreen('start');
    setMFAToken('');
  };

  const startMFA = async (selectedMethod: number) => {
    if (!location) return toaster.error(localLL.errors.locationNotSpecified());
  
    const clientInstances = await clientApi.getInstances();
    const instance = clientInstances.find((i) => i.id === location.instance_id);

    if (!instance) {
      return toaster.error(localLL.errors.instanceNotFound());
    }

    setProxyUrl(instance.proxy_url + CLIENT_MFA_ENDPOINT);
    const mfaStartUrl = instance.proxy_url + CLIENT_MFA_ENDPOINT + '/start';

    // selectedMethod: 0 = authenticator app, 1 = email
    const data = {
      method: selectedMethod,
      pubkey: instance.pubkey,
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

      setScreen(selectedMethod === 0 ? 'authenticator_app' : 'email' );
      setMFAToken(token);

      return response.data;
    } else {
      const error = (response.data as any).error;
      if (error === 'selected MFA method not available') {
        toaster.error(localLL.errors.mfaNotConfigured());
      } else {
        toaster.error(localLL.errors.mfaStartGeneric());
      }

      return;
    }
  };

  const { mutate, isPending } = useMutation({
    mutationFn: startMFA,
  });

  const showEmailCodeForm = useCallback(() => {
    setAuthMethod(1);
    mutate(1);
  }, []);

  const showAuthenticatorAppCodeForm = useCallback(() => {
    setAuthMethod(0);
    mutate(0);
  }, []);

  return (
    <ModalWithTitle
      id="mfa-modal"
      title={localLL.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={resetState}
    >
      {screen === 'start' ? (
        <MFAStart
          isPending={isPending}
          authMethod={authMethod}
          showEmailCodeForm={showEmailCodeForm}
          showAuthenticatorAppCodeForm={showAuthenticatorAppCodeForm}
        />
      ) : (
        <MFACodeForm
          description={screen === 'authenticator_app' ?
            localLL.authenticatorAppDescription() :
            localLL.emailCodeDescription()
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
};

const MFAStart = ({ isPending, authMethod, showAuthenticatorAppCodeForm, showEmailCodeForm }: MFAStartProps) => {
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
          text={isAuthenticatorAppPending ? "" : localLL.useAuthenticatorApp()}
          onClick={showAuthenticatorAppCodeForm}
        />
        <Button
          disabled={isPending}
          size={ButtonSize.LARGE}
          loading={isEmailCodePending}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={isEmailCodePending ? "" : localLL.useEmailCode()}
          onClick={showEmailCodeForm}
        />
      </div>
    </div>
  );
}

type MFACodeForm = {
  description: string;
  token: string;
  proxyUrl: string;
  resetState: () => void;
};

type MFAFinishResponse = {
  preshared_key: string;
};

const MFACodeForm = ({ description, token, proxyUrl, resetState }: MFACodeForm) => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const location = useMFAModal((state) => state.instance);
  const closeModal = useMFAModal((state) => state.close);

  const [error, setError] = useState('');

  const localLL = LL.modals.mfa.authentication;

  const schema = useMemo(
    () =>
      z.object({
        code: z.string().trim().length(6),
      }),
    [LL.form.errors],
  );

  const finishMFA = async (code: string) => {
    if (!location) return toaster.error(localLL.errors.mfaStartGeneric());

    const data = { token, code: Number(code) };

    const response = await fetch<MFAFinishResponse>(proxyUrl + "/finish", {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
      },
      body: Body.json(data),
    });

    if (response.ok) {
      closeModal();

      await connect({
        locationId: location?.id,
        connectionType: location.connection_type,
        presharedKey: response.data.preshared_key,
      });
    } else {
      const { error } = (response.data as any);
      let message = '';
  
      if (error === "Unauthorized") {
        message = localLL.errors.invalidCode();
      } else if (error === "invalid token" || error === "login session not found") {
        console.error(response.data);
        toaster.error(localLL.errors.tokenExpired());
        resetState();
        error(JSON.stringify(response.data));
        return;
      } else {
        toaster.error(localLL.errors.mfaStartGeneric());
      }

      setError(message);
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
          {error ? <MessageBox type={MessageBoxType.ERROR} message={error} /> : null}
        </div>

        <div className="mfa-model-content-footer">
          <Button
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={isPending ? "" : localLL.buttonSubmit()}
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