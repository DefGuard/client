import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { debug, error, info } from '@tauri-apps/plugin-log';
import { useCallback } from 'react';
import { shallow } from 'zustand/shallow';
import { useI18nContext } from '../../../../i18n/i18n-react';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useToaster } from '../../../../shared/defguard-ui/hooks/toasts/useToaster';
import useEffectOnce from '../../../../shared/defguard-ui/utils/useEffectOnce';
import type {
  ActivateUserRequest,
  CreateDeviceResponse,
} from '../../../../shared/hooks/api/types';
import { errorDetail } from '../../../../shared/utils/errorDetail';
import { clientApi } from '../../../client/clientAPI/clientApi';
import { clientQueryKeys } from '../../../client/query';
import { EnrollmentStepKey } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { useEnrollmentApi } from '../../hooks/useEnrollmentApi';

const { saveConfig } = clientApi;

export const SendFinishStep = () => {
  const { LL } = useI18nContext();
  const toaster = useToaster();
  const {
    enrollment: { activateUser },
  } = useEnrollmentApi();
  const queryClient = useQueryClient();

  const finishData = useEnrollmentStore((s) => ({
    phone_number: s.userInfo?.phone_number as string,
    password: s.userPassword as string,
  }));

  const deviceKeys = useEnrollmentStore((s) => s.deviceKeys);
  const deviceResponse = useEnrollmentStore((s) => s.deviceResponse);

  const [setEnrollmentStore] = useEnrollmentStore((state) => [state.setState], shallow);

  const queryFn = useCallback(
    async (
      finishData: ActivateUserRequest,
      deviceResponse: CreateDeviceResponse,
      privateKey: string,
    ) => {
      await activateUser(finishData);
      info('User activated');
      debug('Invoking save_device_config');
      saveConfig({
        privateKey,
        response: deviceResponse,
      })
        .then(() => {
          debug('Config saved');
          setEnrollmentStore({ deviceName: deviceResponse.device.name });
          const invalidate = [clientQueryKeys.getInstances, clientQueryKeys.getLocations];
          invalidate.forEach((key) => {
            queryClient.invalidateQueries({
              queryKey: [key],
            });
          });
        })
        .catch((e) => {
          const detail = errorDetail(e);
          error(`Failed to save config after user activation: ${detail}`);
          if (typeof e === 'string') {
            if (e.includes('Network Error')) {
              toaster.error(LL.common.messages.networkError());
              return;
            }
            toaster.error(LL.common.messages.errorWithMessage({ message: String(e) }));
          } else {
            toaster.error(
              LL.common.messages.errorWithMessage({
                message: String(e),
              }),
            );
          }
        });
    },
    [
      LL.common.messages.errorWithMessage,
      LL.common.messages.networkError,
      activateUser,
      queryClient.invalidateQueries,
      setEnrollmentStore,
      toaster.error,
    ],
  );

  const { mutate } = useMutation({
    mutationFn: () =>
      queryFn(
        finishData,
        deviceResponse as CreateDeviceResponse,
        deviceKeys?.private as string,
      ),
    onError: (e) => {
      setEnrollmentStore({ loading: false });
      toaster.error(
        LL.common.messages.errorWithMessage({
          message: String(e),
        }),
      );
      const detail = errorDetail(e);
      error(`activateUser mutation failed during enrollment finish: ${detail}`);
    },
    onSuccess: () => {
      setEnrollmentStore({ loading: false, step: EnrollmentStepKey.FINISH });
    },
  });

  useEffectOnce(() => {
    setEnrollmentStore({
      loading: true,
    });
    setTimeout(() => {
      mutate();
    }, 250);
  });

  return (
    <Card id="enrollment-finish-request-step">
      <LoaderSpinner size={64} />
    </Card>
  );
};
