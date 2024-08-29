import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useMemo, useState } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { debug, error, info } from 'tauri-plugin-log-api';
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
import { CreateDeviceResponse } from '../../../../../../shared/hooks/api/types';
import { generateWGKeys } from '../../../../../../shared/utils/generateWGKeys';
import { clientApi } from '../../../../../client/clientAPI/clientApi';
import { clientQueryKeys } from '../../../../../client/query';
import { useEnrollmentStore } from '../../../../hooks/store/useEnrollmentStore';
import { useEnrollmentApi } from '../../../../hooks/useEnrollmentApi';

const { saveConfig, saveToken } = clientApi;

type FormFields = {
  name: string;
};

export const DesktopSetup = () => {
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const stepLL = LL.pages.enrollment.steps.deviceSetup;
  const {
    enrollment: { createDevice, activateUser },
  } = useEnrollmentApi();
  const deviceName = useEnrollmentStore((state) => state.deviceName);
  const [userInfo, userPassword] = useEnrollmentStore((state) => [
    state.userInfo,
    state.userPassword,
  ]);
  const setEnrollmentStore = useEnrollmentStore((state) => state.setState);
  const next = useEnrollmentStore((state) => state.nextStep);
  const [isLoading, setIsLoading] = useState(false);

  const { mutateAsync: mutateUserActivation, isPending: activationPending } = useMutation(
    {
      mutationFn: activateUser,
      onError: (e) => {
        toaster.error(LL.common.messages.error());
        console.error(e);
        error(String(e));
      },
    },
  );

  const { mutateAsync: createDeviceMutation, isPending: createDevicePending } =
    useMutation({
      mutationFn: createDevice,
      onError: (e) => {
        error(String(e));
      },
    });

  const { mutateAsync: saveTokenMutation, isPending: saveTokenPending } = useMutation({
    mutationFn: saveToken,
    onError: (e) => {
      toaster.error(LL.common.messages.error());
      console.error(e);
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
    const deviceResponse = await createDeviceMutation({
      name: values.name,
      pubkey: publicKey,
    }).then((res) => {
      if (!res.ok) {
        error(
          `Failed to create device during the enrollment. Error details: ${JSON.stringify(
            res.data,
          )} Error status code: ${JSON.stringify(res.status)}`,
        );
        throw Error('Failed to create device');
      }
      return res;
    });
    mutateUserActivation({
      password: userPassword,
      phone_number: userInfo.phone_number,
    }).then((res) => {
      if (!res.ok) {
        error(
          `Failed to activate user during the enrollment. Error details: ${JSON.stringify(
            res.data,
          )} Error status code: ${JSON.stringify(res.status)}`,
        );
        throw Error('Failed to activate user');
      }
      info('User activated');
      debug('Invoking save_device_config');
      saveConfig({
        privateKey,
        token: res.data.token,
        response: deviceResponse.data as CreateDeviceResponse,
      })
        .then(() => {
          debug('Config saved');
          setIsLoading(false);
          setEnrollmentStore({ deviceName: values.name });
          toaster.success(stepLL.desktopSetup.messages.deviceConfigured());
          const invalidate = [
            clientQueryKeys.getInstances,
            clientQueryKeys.getLocations,
          ];
          invalidate.forEach((key) => {
            queryClient.invalidateQueries({
              queryKey: [key],
            });
          });
          next();
        })
        .catch((e) => {
          setIsLoading(false);

          if (typeof e === 'string') {
            if (e.includes('Network Error')) {
              toaster.error(LL.common.messages.networkError());
              return;
            }
            toaster.error(LL.common.messages.error());
          } else {
            toaster.error((e as Error).message);
          }
        });
    });
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
          loading={isLoading || activationPending || createDevicePending || saveTokenPending}
        />
      </form>
    </Card>
  );
};
