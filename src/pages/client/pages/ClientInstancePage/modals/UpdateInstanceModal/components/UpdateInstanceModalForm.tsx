import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { fetch } from '@tauri-apps/plugin-http';
import { useMemo } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { useToaster } from '../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import type {
  CreateDeviceResponse,
  EnrollmentStartResponse,
} from '../../../../../../../shared/hooks/api/types';
import { clientApi } from '../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../../query';
import { useDeleteInstanceModal } from '../../DeleteInstanceModal/useDeleteInstanceModal';
import { useUpdateInstanceModal } from '../useUpdateInstanceModal';

const { updateInstance } = clientApi;

type FormFields = {
  token: string;
  url: string;
};

const invalidateOnSuccess = [clientQueryKeys.getInstances, clientQueryKeys.getLocations];

export const UpdateInstanceModalForm = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.updateInstance;
  const instance = useUpdateInstanceModal((state) => state.instance);
  const openDeleteInstance = useDeleteInstanceModal((state) => state.open);
  const closeModal = useUpdateInstanceModal((state) => state.close);
  const toaster = useToaster();
  const queryClient = useQueryClient();
  const setClientState = useClientStore((s) => s.setState, shallow);

  const defaultValues = useMemo(
    (): FormFields => ({
      url: instance ? instance.proxy_url : '',
      token: '',
    }),
    [instance],
  );

  const schema = useMemo(
    () =>
      z.object({
        url: z.string().min(1, LL.form.errors.required()),
        token: z.string().min(1, LL.form.errors.required()),
      }),
    [LL.form.errors],
  );

  const {
    handleSubmit,
    control,
    formState: { isSubmitting },
    setError,
  } = useForm<FormFields>({
    defaultValues,
    mode: 'all',
    resolver: zodResolver(schema),
  });

  const onValidSubmit: SubmitHandler<FormFields> = async (values) => {
    const clientInstances = await clientApi.getInstances();
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

    const res = await fetch<EnrollmentStartResponse>(endpointUrl, {
      method: 'POST',
      headers,
      body: JSON.stringify(data),
    });
    if (res.ok) {
      const enrollmentData = res.data;
      let proxy_api_url = values.url;
      if (proxy_api_url[proxy_api_url.length - 1] === '/') {
        proxy_api_url = proxy_api_url.slice(0, -1);
      }
      proxy_api_url = `${proxy_api_url}/api/v1`;
      const instance = clientInstances.find((i) => i.uuid === enrollmentData.instance.id);
      if (instance) {
        const authCookie = res.headers.getSetCookie().find((cookie) =>
          cookie.startsWith('defguard_proxy='),
        );
        if (!authCookie) {
          toaster.error(
            LL.common.messages.errorWithMessage({
              message: LL.common.messages.noCookie(),
            }),
          );
          return;
        }
        headers.Cookie = authCookie;
        const instanceInfoResponse = await fetch<CreateDeviceResponse>(
          `${proxy_api_url}/enrollment/network_info`,
          {
            method: 'POST',
            headers,
            body: JSON.stringify({
              pubkey: instance.pubkey,
            }),
          },
        );
        if (instanceInfoResponse.ok) {
          updateInstance({
            instanceId: instance.id,
            response: instanceInfoResponse.data,
          })
            .then(() => {
              invalidateOnSuccess.forEach((k) => {
                queryClient.invalidateQueries({
                  type: 'active',
                  queryKey: [k],
                });
              });
              toaster.success(
                localLL.messages.success({
                  name: instance.name,
                }),
              );
              closeModal();
            })
            .catch(() => {
              toaster.error(LL.common.messages.error());
            });
        }
      } else {
        // Instance not found in client, use add instance.
        toaster.error(localLL.messages.errorInstanceNotFound());
        setError(
          'token',
          {
            message: localLL.form.fieldErrors.token.instanceIsNotPresent(),
          },
          {
            shouldFocus: true,
          },
        );
      }
    } else {
      // Token or URL is invalid.
      toaster.error(
        LL.common.messages.errorWithMessage({
          message: 'Token or URL is invalid',
        }),
      );
      setError(
        'token',
        {
          message: localLL.form.fieldErrors.token.rejected(),
        },
        { shouldFocus: false },
      );
    }
  };

  return (
    <form data-testid="update-instance-modal-form" onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput
        label={localLL.form.fieldLabels.url()}
        controller={{ control, name: 'url' }}
        disabled={isSubmitting}
      />
      <FormInput
        label={localLL.form.fieldLabels.token()}
        controller={{ control, name: 'token' }}
        disabled={isSubmitting}
      />
      <div className="controls">
        <Button
          type="submit"
          styleVariant={ButtonStyleVariant.PRIMARY}
          size={ButtonSize.LARGE}
          text={localLL.controls.updateInstance()}
          loading={isSubmitting}
        />
        <Button
          type="button"
          styleVariant={ButtonStyleVariant.DELETE}
          size={ButtonSize.LARGE}
          text={localLL.controls.removeInstance()}
          onClick={() => {
            if (instance) {
              setClientState({
                selectedInstance: undefined,
                selectedLocation: undefined,
              });
              openDeleteInstance(instance);
            }
          }}
        />
      </div>
    </form>
  );
};
