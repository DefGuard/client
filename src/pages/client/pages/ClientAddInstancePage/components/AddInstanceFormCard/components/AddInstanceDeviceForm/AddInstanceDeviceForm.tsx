import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { fetch } from '@tauri-apps/plugin-http';
import { error } from '@tauri-apps/plugin-log';
import { hostname } from '@tauri-apps/plugin-os';
import { useCallback, useMemo, useState } from 'react';
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
  CreateDeviceRequest,
  CreateDeviceResponse,
} from '../../../../../../../../shared/hooks/api/types';
import { routes } from '../../../../../../../../shared/routes';
import { generateWGKeys } from '../../../../../../../../shared/utils/generateWGKeys';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { type SelectedInstance, WireguardInstanceType } from '../../../../../../types';
import { useAddInstanceStore } from '../../../../hooks/useAddInstanceStore';
import type { AddInstanceInitResponse } from '../../types';

const { getInstances, saveConfig } = clientApi;

type ErrorData = {
  error: string;
};

export const AddInstanceDeviceForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.forms.device;
  const toaster = useToaster();
  const setClientStore = useClientStore((state) => state.setState);
  const navigate = useNavigate();
  const [isLoading, setIsLoading] = useState(false);
  const response = useAddInstanceStore((s) => s.response as AddInstanceInitResponse);
  const resetAddInstanceStore = useAddInstanceStore((s) => s.reset);

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

  type FormFields = z.infer<typeof schema>;

  const defaultValues = useCallback(async (): Promise<FormFields> => {
    const name = await hostname();
    return {
      name: name ?? '',
    };
  }, []);

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
        body: JSON.stringify(data),
        method: 'POST',
      }).then(async (r) => {
        if (!r.ok) {
          setIsLoading(false);
          const data = (await r.json()) as ErrorData;
          const details = `${data?.error ? `${data.error}, ` : ''}`;
          error(
            `Failed to create device. Check enrollment and Defguard logs, details: ${details}. Error status code: ${r.status}`,
          );
          throw Error(`Failed to create device, details: ${details} `);
        }
        const deviceResp = (await r.json()) as CreateDeviceResponse;
        saveConfig({
          privateKey: privateKey,
          response: deviceResp,
        })
          .then(async (res) => {
            setIsLoading(false);
            toaster.success(localLL.messages.addSuccess());
            const instances = await getInstances();
            const selectedInstance: SelectedInstance = {
              id: res.instance.id,
              type: WireguardInstanceType.DEFGUARD_INSTANCE,
            };
            setClientStore({ selectedInstance, instances });
            setTimeout(() => {
              resetAddInstanceStore();
            }, 250);
            navigate(routes.client.instancePage, { replace: true });
          })
          .catch((e) => {
            toaster.error(
              LL.common.messages.errorWithMessage({
                message: String(e),
              }),
            );
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
            styleVariant={ButtonStyleVariant.STANDARD}
            type="button"
            text={LL.common.controls.cancel()}
            disabled={isLoading}
            onClick={() => navigate(routes.client.instancePage)}
          />
          <Button
            size={ButtonSize.LARGE}
            styleVariant={ButtonStyleVariant.PRIMARY}
            type="submit"
            text={localLL.submit()}
            loading={isLoading}
          />
        </div>
      </form>
    </>
  );
};
