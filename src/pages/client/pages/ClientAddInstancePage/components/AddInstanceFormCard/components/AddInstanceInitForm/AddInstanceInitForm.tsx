import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { Body, fetch, Response } from '@tauri-apps/api/http';
import { invoke } from '@tauri-apps/api/tauri';
import dayjs from 'dayjs';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { debug, error, info } from 'tauri-plugin-log-api';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useToaster } from '../../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import {
  CreateDeviceResponse,
  EnrollmentError,
  EnrollmentStartResponse,
} from '../../../../../../../../shared/hooks/api/types';
import { routes } from '../../../../../../../../shared/routes';
import { useEnrollmentStore } from '../../../../../../../enrollment/hooks/store/useEnrollmentStore';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientFlags } from '../../../../../../hooks/useClientFlags';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { SelectedInstance, WireguardInstanceType } from '../../../../../../types';
import { AddInstanceInitResponse } from '../../types';

type Props = {
  nextStep: (data: AddInstanceInitResponse) => void;
};

type FormFields = {
  url: string;
  token: string;
};

const defaultValues: FormFields = {
  url: '',
  token: '',
};

export const AddInstanceInitForm = ({ nextStep }: Props) => {
  const toaster = useToaster();
  const navigate = useNavigate();
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.forms.initInstance;
  const [isLoading, setIsLoading] = useState(false);
  const initEnrollment = useEnrollmentStore((state) => state.init);
  const setClientState = useClientStore((state) => state.setState);
  const setClientFlags = useClientFlags((state) => state.setValues);

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

  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    debug('Sending token to proxy');
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
    fetch<EnrollmentStartResponse>(endpointUrl, {
      method: 'POST',
      headers,
      body: Body.json(data),
    })
      .then(async (res: Response<EnrollmentStartResponse | EnrollmentError>) => {
        const authCookie = res.headers['set-cookie'];
        if (!res.ok) {
          setIsLoading(false);
          error(JSON.stringify(res.data));
          error(JSON.stringify(res.status));
          const errorMessage = (res.data as EnrollmentError).error;

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
        debug('Response received with status OK');
        const r = res.data as EnrollmentStartResponse;
        // get client registered instances
        const clientInstances = await clientApi.getInstances();
        const instance = clientInstances.find((i) => i.uuid === r.instance.id);
        let proxy_api_url = values.url;
        if (proxy_api_url[proxy_api_url.length - 1] === '/') {
          proxy_api_url = proxy_api_url.slice(0, -1);
        }
        proxy_api_url = proxy_api_url + '/api/v1';
        setIsLoading(false);

        if (instance) {
          debug('Instance already exists, fetching update');
          // update already registered instance instead
          headers['Cookie'] = authCookie;
          fetch<CreateDeviceResponse>(`${proxy_api_url}/enrollment/network_info`, {
            method: 'POST',
            headers,
            body: Body.json({
              pubkey: instance.pubkey,
            }),
          }).then(async (res) => {
            invoke<void>('update_instance', {
              instanceId: instance.id,
              response: res.data,
            })
              .then(() => {
                info('Configured device');
                toaster.success(
                  LL.pages.enrollment.steps.deviceSetup.desktopSetup.messages.deviceConfigured(),
                );
                const _selectedInstace: SelectedInstance = {
                  id: instance.id,
                  type: WireguardInstanceType.DEFGUARD_INSTANCE,
                };
                setClientFlags({
                  selectedLocation: 0,
                  selectedInstance: _selectedInstace,
                });
                setClientState({
                  selectedInstance: _selectedInstace,
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
        if (r.user.enrolled) {
          //no, only create new device for desktop client
          debug('User already active, adding device only.');
          nextStep({
            url: proxy_api_url,
            cookie: authCookie,
            device_names: r.user.device_names,
          });
        } else {
          // yes, enroll the user
          debug('User is not active. Starting enrollment.');
          const sessionEnd = dayjs.unix(r.deadline_timestamp).utc().local().format();
          const sessionStart = dayjs().local().format();
          initEnrollment({
            userInfo: r.user,
            adminInfo: r.admin,
            endContent: r.final_page_content,
            proxy_url: proxy_api_url,
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
