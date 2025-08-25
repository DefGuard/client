import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation } from '@tanstack/react-query';
import { error } from '@tauri-apps/plugin-log';
import { isUndefined } from 'lodash-es';
import { useMemo, useState } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { Card } from '../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import type { CreateDeviceResponse } from '../../../../../../shared/hooks/api/types';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { useEnrollmentStore } from '../../../../hooks/store/useEnrollmentStore';
import { useEnrollmentApi } from '../../../../hooks/useEnrollmentApi';

type FormFields = {
  name: string;
};

export const DesktopSetup = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const stepLL = LL.pages.enrollment.steps.deviceSetup;
  const {
    enrollment: { createDevice },
  } = useEnrollmentApi();
  const deviceName = useEnrollmentStore((state) => state.deviceName);
  const [userInfo, userPassword] = useEnrollmentStore((state) => [
    state.userInfo,
    state.userPassword,
  ]);
  const setEnrollmentStore = useEnrollmentStore((state) => state.setState);
  const next = useEnrollmentStore((state) => state.nextStep);
  const [isLoading, setIsLoading] = useState(false);

  const { mutateAsync: createDeviceMutation, isPending: createDevicePending } =
    useMutation({
      mutationFn: createDevice,
      onError: (e) => {
        error(String(e));
      },
    });

  const schema = useMemo(
    () =>
      z.object({
        name: z.string().trim().min(1, LL.form.errors.required()),
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
    const deviceResponse = (await createDeviceMutation({
      name: values.name,
      pubkey: publicKey,
    }).then(async (res) => {
      if (!res.ok) {
        error(
          `Failed to create device during the enrollment. Error details: ${JSON.stringify(
            await res.json(),
          )} Error status code: ${res.status} `,
        );
      }
      return res;
    })) as CreateDeviceResponse;
    toaster.success(stepLL.desktopSetup.messages.deviceConfigured());
    setEnrollmentStore({
      deviceName: values.name,
      deviceKeys: {
        private: privateKey,
        public: publicKey,
      },
      deviceResponse,
    });
    setIsLoading(false);
    next();
  };

  return (
    <Card id="desktop-device-setup">
      <h3>{stepLL.desktopSetup.title()}</h3>
      <form onSubmit={handleSubmit(handleValidSubmit)}>
        <FormInput
          controller={{ control, name: 'name' }}
          label={LL.pages.enrollment.steps.deviceSetup.cards.device.create.form.fields.name.label()}
          disabled={!isUndefined(deviceName)}
        />
        <Button
          type="submit"
          size={ButtonSize.LARGE}
          styleVariant={deviceName ? ButtonStyleVariant.SAVE : ButtonStyleVariant.PRIMARY}
          text={
            deviceName
              ? stepLL.desktopSetup.controls.success()
              : stepLL.desktopSetup.controls.create()
          }
          disabled={!isUndefined(deviceName)}
          loading={isLoading || createDevicePending}
        />
      </form>
    </Card>
  );
};
