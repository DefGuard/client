import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { Body, fetch } from '@tauri-apps/api/http';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { useNavigate } from 'react-router-dom';
import { error } from 'tauri-plugin-log-api';
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
  CreateDeviceRequest,
  CreateDeviceResponse,
} from '../../../../../../../../shared/hooks/api/types';
import { routes } from '../../../../../../../../shared/routes';
import { generateWGKeys } from '../../../../../../../../shared/utils/generateWGKeys';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientFlags } from '../../../../../../hooks/useClientFlags';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { SelectedInstance, WireguardInstanceType } from '../../../../../../types';
import { AddInstanceInitResponse } from '../../types';

const { getInstances, saveConfig } = clientApi;

type Props = {
  response: AddInstanceInitResponse;
};

type FormFields = {
  name: string;
};

type ErrorData = {
  error: string;
};

const defaultValues: FormFields = {
  name: '',
};

export const AddInstanceDeviceForm = ({ response }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.forms.device;
  const toaster = useToaster();
  const setClientStore = useClientStore((state) => state.setState);
  const setCliengFlags = useClientFlags((state) => state.setValues);
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = useState(false);

  const { url: proxyUrl, cookie, device_names } = response;

  const schema = useMemo(
    () =>
      z.object({
        name: z
          .string()
          .trim()
          .min(1, LL.form.errors.required())
          .refine((val) => !device_names.includes(val), {
            message: LL.form.errors.duplicatedName(),
          }),
      }),
    [LL.form.errors, device_names],
  );

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues: defaultValues,
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    if (!proxyUrl) return;
    setIsLoading(true);
    const { publicKey, privateKey } = generateWGKeys();
    const data: CreateDeviceRequest = {
      name: values.name,
      pubkey: publicKey,
    };
    const headers = {
      'Content-Type': 'application/json',
      Cookie: cookie,
    };
    try {
      await fetch(`${proxyUrl}/enrollment/create_device`, {
        headers,
        body: Body.json(data),
        method: 'POST',
      }).then((r) => {
        if (!r.ok) {
          setIsLoading(false);
          const details = `${
            (r.data as ErrorData)?.error ? (r.data as ErrorData).error + ', ' : ''
          }`;
          error(
            `Failed to create device check enrollment and defguard logs, details: ${details}Error status code: ${r.status}`,
          );
          throw Error(`Failed to create device, details: ${details}`);
        }
        const deviceResp = r.data as CreateDeviceResponse;
        saveConfig({
          privateKey: privateKey,
          response: deviceResp,
        })
          .then(async (res) => {
            setIsLoading(false);
            toaster.success(localLL.messages.addSuccess());
            const instances = await getInstances();
            const _selectedInstance: SelectedInstance = {
              id: res.instance.id,
              type: WireguardInstanceType.DEFGUARD_INSTANCE,
            };
            setCliengFlags({
              selectedLocation: 0,
              selectedInstance: _selectedInstance,
            });
            setClientStore({
              selectedInstance: _selectedInstance,
              instances,
            });
            navigate(routes.client.instancePage, { replace: true });
          })
          .catch(() => {
            toaster.error(LL.common.messages.error());
            setIsLoading(false);
          });
      });
    } catch (e) {
      setIsLoading(false);
      console.error(e);

      if (typeof e === 'string') {
        if (e.includes('Network Error')) {
          toaster.error(LL.common.messages.networkError());
          return;
        }
        toaster.error(LL.common.messages.error());
      } else {
        toaster.error((e as Error).message);
      }
    }
  };

  return (
    <>
      <h2>{localLL.title()}</h2>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput controller={{ control, name: 'name' }} label={localLL.labels.name()} />
        <div className="controls">
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            type="submit"
            text={localLL.submit()}
            loading={isLoading}
          />
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.STANDARD}
            type="button"
            text={LL.common.controls.cancel()}
            loading={isLoading}
            onClick={() => navigate(routes.client.instancePage)}
          />
        </div>
      </form>
    </>
  );
};
