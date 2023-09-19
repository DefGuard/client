import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { invoke } from '@tauri-apps/api/tauri';
import { isUndefined } from 'lodash-es';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { CreateDeviceResponse } from '../../../../../../shared/hooks/api/types';
import { useApi } from '../../../../../../shared/hooks/api/useApi';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { useEnrollmentStore } from '../../../../hooks/store/useEnrollmentStore';

type FormFields = {
  name: string;
};

const saveConfig = async (
  privateKey: string,
  response: CreateDeviceResponse,
): Promise<void> =>
  invoke<void>('save_device_config', {
    privateKey: privateKey,
    response,
  });

export const DekstopSetup = () => {
  const { LL } = useI18nContext();
  const {
    enrollment: { createDevice, activateUser },
  } = useApi();
  const deviceName = useEnrollmentStore((state) => state.deviceName);
  const [userInfo, userPassword] = useEnrollmentStore((state) => [
    state.userInfo,
    state.userPassword,
  ]);
  const setEnrollmentStore = useEnrollmentStore((state) => state.setState);
  const [isLoading, setIsLoading] = useState(false);

  const { isLoading: loadingUserActivation, mutateAsync: mutateUserActivation } =
    useMutation({
      mutationFn: activateUser,
      onError: (e) => {
        console.error(e);
      },
    });

  const { isLoading: loadingCreateDevice, mutateAsync: createDeviceMutation } =
    useMutation({
      mutationFn: createDevice,
      onError: () => {},
    });

  const schema = useMemo(
    () =>
      z.object({
        name: z.string().trim().nonempty(LL.form.errors.required()),
      }),
    [LL.form.errors],
  );

  const { control, handleSubmit } = useForm<FormFields>({
    mode: 'all',
    defaultValues: {
      name: deviceName ?? '',
    },
    resolver: zodResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    if (!userInfo || !userPassword) return;
    const { publicKey, privateKey } = generateWGKeys();
    const deviceResponse = await createDeviceMutation({
      name: values.name,
      pubkey: publicKey,
    });
    mutateUserActivation({
      password: userPassword,
      phone_number: userInfo.phone_number,
    }).then(() => {
      console.log('User activated');
      setIsLoading(true);
      saveConfig(privateKey, deviceResponse)
        .then(() => {
          console.log('config saved');
          setIsLoading(false);
          setEnrollmentStore({ deviceName: values.name });
        })
        .catch((e) => {
          console.log('Failed to save config');
          console.log({
            privateKey,
            deviceResponse,
          });
          setIsLoading(false);
          console.error(e);
        });
    });
  };

  return (
    <Card id="desktop-device-setup">
      <h3>{LL.pages.enrollment.steps.deviceSetup.cards.device.title()}</h3>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'name' }}
          label={LL.pages.enrollment.steps.deviceSetup.cards.device.create.form.fields.name.label()}
          disabled={!isUndefined(deviceName)}
        />
        <Button
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.pages.enrollment.steps.deviceSetup.cards.device.create.submit()}
          disabled={!isUndefined(deviceName)}
          loading={isLoading || loadingUserActivation || loadingCreateDevice}
        />
      </form>
    </Card>
  );
};
