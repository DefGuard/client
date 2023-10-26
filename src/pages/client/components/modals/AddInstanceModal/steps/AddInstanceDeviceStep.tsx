import { zodResolver } from '@hookform/resolvers/zod';
import { Body, fetch } from '@tauri-apps/api/http';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { error } from 'tauri-plugin-log-api';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import {
  CreateDeviceRequest,
  CreateDeviceResponse,
} from '../../../../../../shared/hooks/api/types';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useAddInstanceModal } from '../hooks/useAddInstanceModal';

const { saveConfig } = clientApi;

type FormFields = {
  name: string;
};

const defaultValues: FormFields = {
  name: '',
};

export const AddInstanceDeviceStep = () => {
  const { LL } = useI18nContext();
  const componentLL = LL.pages.client.modals.addInstanceModal;
  const toaster = useToaster();
  const close = useAddInstanceModal((state) => state.close);
  const [isLoading, setIsLoading] = useState(false);

  const [proxyUrl, cookie] = useAddInstanceModal(
    (state) => [state.proxyUrl, state.cookie],
    shallow,
  );

  const schema = useMemo(
    () => z.object({ name: z.string().trim().min(1, LL.form.errors.required()) }),
    [LL.form.errors],
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
          .then(() => {
            setIsLoading(false);
            toaster.success(componentLL.messages.success.add());
            close();
          })
          .catch(() => {
            toaster.error(LL.common.messages.error());
            setIsLoading(false);
            close();
          });
      });
    } catch (e) {
      setIsLoading(false);
      toaster.error(LL.common.messages.error());
      close();
      console.error(e);
    }
  };

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput
        controller={{ control, name: 'name' }}
        label={componentLL.form.fields.name.label()}
      />
      <div className="controls">
        <Button
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={componentLL.form.submit.addDevice()}
          loading={isLoading}
        />
      </div>
    </form>
  );
};
