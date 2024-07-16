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
import { useClientStore } from '../../../../../../hooks/useClientStore';
import { WireguardInstanceType } from '../../../../../../types';
import { AddInstanceInitResponse } from '../../types';

const { getInstances, saveConfig } = clientApi;

type Props = {
  response: AddInstanceInitResponse;
};

type FormFields = {
  name: string;
};

const defaultValues: FormFields = {
  name: '',
};

export const AddInstanceDeviceForm = ({ response }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.addInstancePage.forms.device;
  const toaster = useToaster();
  const setClientStore = useClientStore((state) => state.setState);
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
          toaster.error(LL.common.messages.error());
          error('Failed to create device check enrollment and defguard logs');
          throw Error('Failed to create device');
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
            setClientStore({
              selectedInstance: {
                id: res.instance.id,
                type: WireguardInstanceType.DEFGUARD_INSTANCE,
              },
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
      toaster.error(LL.common.messages.error());
      console.error(e);
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
        </div>
      </form>
    </>
  );
};
