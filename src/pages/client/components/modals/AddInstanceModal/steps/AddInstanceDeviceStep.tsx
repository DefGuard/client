import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
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
import { clientQueryKeys } from '../../../../query';
import { useAddInstanceModal } from '../hooks/useAddInstanceModal';

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
  const queryClient = useQueryClient();

  const [proxyUrl] = useAddInstanceModal((state) => [state.proxyUrl], shallow);

  const schema = useMemo(
    () => z.object({ name: z.string().trim().nonempty(LL.form.errors.required()) }),
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
    };
    try {
      await fetch(`${proxyUrl}/enrollment/create_device`, {
        headers,
        body: JSON.stringify(data),
        method: 'POST',
      }).then((r) => {
        if (!r.ok) {
          setIsLoading(false);
          toaster.error(LL.common.messages.error());
          throw Error('Failed to create device');
        }
        r.json().then((deviceResp: CreateDeviceResponse) => {
          invoke('save_device_config', {
            privateKey: privateKey,
            response: deviceResp,
          })
            .then(() => {
              setIsLoading(false);
              toaster.success(componentLL.messages.success.add());
              queryClient.invalidateQueries([clientQueryKeys.getInstances]);
              queryClient.invalidateQueries([clientQueryKeys.getLocations]);
              close();
            })
            .catch((e) => {
              toaster.error(LL.common.messages.error());
              setIsLoading(false);
              close();
              console.error(e);
            });
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
