import './style.scss';

import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useClientFlags } from '../../../../hooks/useClientFlags';
import { clientQueryKeys } from '../../../../query';
import { useUpdateInstanceModal } from '../UpdateInstanceModal/useUpdateInstanceModal';
import { useDeleteInstanceModal } from './useDeleteInstanceModal';

const { deleteInstance } = clientApi;

const invalidateOnSuccess = [
  clientQueryKeys.getInstances,
  clientQueryKeys.getLocations,
  clientQueryKeys.getConnections,
];

export const DeleteInstanceModal = () => {
  const { LL } = useI18nContext();
  const [isOpen, instance] = useDeleteInstanceModal(
    (state) => [state.isOpen, state.instance],
    shallow,
  );
  const [close, reset] = useDeleteInstanceModal(
    (state) => [state.close, state.reset],
    shallow,
  );
  const [closeUpdate] = useUpdateInstanceModal((state) => [state.close], shallow);
  const toaster = useToaster();
  const localLL = LL.modals.deleteInstance;
  const queryClient = useQueryClient();
  const setClientFlags = useClientFlags((state) => state.setValues);

  const { mutate, isPending } = useMutation({
    mutationFn: deleteInstance,
    onSuccess: () => {
      toaster.success(localLL.messages.success());
      invalidateOnSuccess.forEach((key) => {
        queryClient.invalidateQueries({
          queryKey: [key],
          refetchType: 'active',
        });
      });
    },
    onError: (e) => {
      toaster.error(localLL.messages.error());
      console.error(e);
    },
  });

  // reset state on mount
  useEffect(() => {
    reset();
    // eslint-disable-next-line
  }, []);

  return (
    <ConfirmModal
      id="delete-instance-modal"
      title={localLL.title()}
      subTitle={localLL.subtitle({ name: instance?.name ?? '' })}
      type={ConfirmModalType.WARNING}
      isOpen={isOpen && !isUndefined(instance)}
      onClose={() => close()}
      afterClose={() => reset()}
      loading={isPending}
      submitText={localLL.controls.submit()}
      cancelText={LL.common.controls.cancel()}
      onSubmit={() => {
        if (instance) {
          setClientFlags({
            selectedInstance: undefined,
            selectedLocation: undefined,
          });
          mutate(instance.id);
          close();
          closeUpdate();
        }
      }}
      onCancel={() => close()}
    />
  );
};
