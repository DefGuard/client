import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { invoke } from '@tauri-apps/api/core';
import { fetch } from '@tauri-apps/plugin-http';
import { debug, error, info } from '@tauri-apps/plugin-log';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useToaster } from '../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import type {
  CreateDeviceResponse,
  EnrollmentError,
  EnrollmentStartResponse,
} from '../../../../../../../../shared/hooks/api/types';
import { routes } from '../../../../../../../../shared/routes';
import { useEnrollmentStore } from '../../../../../../../enrollment/hooks/store/useEnrollmentStore';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { type SelectedInstance, WireguardInstanceType } from '../../../../../../types';
import { AddInstanceFormStep } from '../../../../hooks/types';
import { useAddInstanceStore } from '../../../../hooks/useAddInstanceStore';

export const AddInstanceInitForm = () => {
  const setPageState = useAddInstanceStore((s) => s.setState);
  const toaster = useToaster();
  const navigate = useNavigate();
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.forms.initInstance;
  const [isLoading, setIsLoading] = useState(false);
  const initEnrollment = useEnrollmentStore((state) => state.init);
  const setClientState = useClientStore((state) => state.setState);

  const schema = useMemo(
    () =>
      z.object({
        url: z
          .string()
          .trim()
          .min(1, LL.form.errors.required())
          .url(LL.form.errors.invalid()),
        token: z.string().trim().min(1, LL.form.errors.required()),
      }),
    [LL.form.errors],
  );

  type FormFields = z.infer<typeof schema>;

  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues: {
      url: '',
      token: '',
    },
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    const url = () => {
      const endpoint = '/api/v1/enrollment/start';
      let base: string;
      if (values.url[values.url.length - 1] === '/') {
        base = values.url.slice(0, -1);
      } else {
        base = values.url;
      }
      return base + endpoint;
    };

    const endpointUrl = url();

    const headers: Record<string, string> = {
      'Content-Type': 'application/json',
    };

    const data = {
      token: values.token,
    };

    setIsLoading(true);
    fetch(endpointUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify(data),
    })
      .then(async (res: Response) => {
        if (!res.ok) {
          setIsLoading(false);
          error(JSON.stringify(res.status));
          const errorMessage = ((await res.json()) as EnrollmentError).error;

          switch (errorMessage) {
            case 'token expired': {
              throw Error(LL.common.messages.tokenExpired());
            }
            default: {
              throw Error(
                LL.common.messages.errorWithMessage({
                  message: errorMessage,
                }),
              );
            }
          }
        }
        // There may be other set-cookies, set by e.g. a proxy
        // Get only the defguard_proxy cookie
        const authCookie = res.headers
          .getSetCookie()
          .find((cookie) => cookie.startsWith('defguard_proxy='));
        if (!authCookie) {
          setIsLoading(false);
          error(
            LL.common.messages.errorWithMessage({
              message: LL.common.messages.noCookie(),
            }),
          );
          throw Error(
            LL.common.messages.errorWithMessage({
              message: LL.common.messages.noCookie(),
            }),
          );
        }
        debug('Response received with status OK');
        const startResponse = (await res.json()) as EnrollmentStartResponse;
        // get client registered instances
        const clientInstances = await clientApi.getInstances();
        const instance = clientInstances.find(
          (i) => i.uuid === startResponse.instance.id,
        );
        let proxy_api_url = values.url;
        if (proxy_api_url[proxy_api_url.length - 1] === '/') {
          proxy_api_url = proxy_api_url.slice(0, -1);
        }
        proxy_api_url = `${proxy_api_url}/api/v1`;
        setIsLoading(false);

        if (instance) {
          debug('Instance already exists, fetching update');
          // update already registered instance instead
          headers.Cookie = authCookie;
          fetch(`${proxy_api_url}/enrollment/network_info`, {
            method: 'POST',
            headers,
            body: JSON.stringify({
              pubkey: instance.pubkey,
            }),
          }).then(async (res) => {
            invoke<void>('update_instance', {
              instanceId: instance.id,
              response: (await res.json()) as CreateDeviceResponse,
            })
              .then(() => {
                info('Configured device');
                toaster.success(
                  LL.pages.enrollment.steps.deviceSetup.desktopSetup.messages.deviceConfigured(),
                );
                const _selectedInstance: SelectedInstance = {
                  id: instance.id,
                  type: WireguardInstanceType.DEFGUARD_INSTANCE,
                };
                setClientState({
                  selectedInstance: _selectedInstance,
                });
                navigate(routes.client.base, { replace: true });
              })
              .catch((e) => {
                error(e);
                toaster.error(
                  LL.common.messages.errorWithMessage({
                    message: String(e),
                  }),
                );
              });
          });
        }
        // register new instance
        // is user in need of full enrollment ?
        if (startResponse.user.enrolled) {
          //no, only create new device for desktop client
          debug('User already active, adding device only.');
          setPageState({
            step: AddInstanceFormStep.DEVICE,
            response: {
              url: proxy_api_url,
              cookie: authCookie,
              device_names: startResponse.user.device_names,
            },
          });
        } else {
          // yes, enroll the user
          debug('User is not active. Starting enrollment.');
          const sessionEnd = dayjs
            .unix(startResponse.deadline_timestamp)
            .utc()
            .local()
            .format();
          const sessionStart = dayjs().local().format();
          initEnrollment({
            userInfo: startResponse.user,
            adminInfo: startResponse.admin,
            endContent: startResponse.final_page_content,
            proxy_url: proxy_api_url,
            enrollmentSettings: startResponse.settings,
            sessionEnd,
            sessionStart,
            cookie: authCookie,
          });
          navigate(routes.enrollment, { replace: true });
        }
      })
      .catch((e) => {
        setIsLoading(false);
        if (typeof e === 'string') {
          if (e.includes('Network Error')) {
            toaster.error(LL.common.messages.networkError());
            return;
          }
          toaster.error(
            LL.common.messages.errorWithMessage({
              message: String(e),
            }),
          );
        } else {
          toaster.error(
            LL.common.messages.errorWithMessage({
              message: (e as Error).message,
            }),
          );
        }
      });
  };

  return (
    <>
      <h2>{localLL.title()}</h2>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput controller={{ control, name: 'url' }} label={localLL.labels.url()} />
        <FormInput
          controller={{ control, name: 'token' }}
          label={localLL.labels.token()}
        />
        <div className="controls">
          <Button
            type="submit"
            className="submit"
            loading={isLoading}
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            text={localLL.submit()}
          />
        </div>
      </form>
    </>
  );
};
