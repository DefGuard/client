import { useMutation, useQueryClient } from '@tanstack/react-query';
import { isUndefined } from 'lodash-es';
import { useNavigate } from 'react-router-dom';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { ConfirmModal } from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/ConfirmModal';
import { ConfirmModalType } from '../../../../../../shared/defguard-ui/components/Layout/modals/ConfirmModal/types';
import { useToaster } from '../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { routes } from '../../../../../../shared/routes';
import { clientApi } from '../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../query';
import { WireguardInstanceType } from '../../../../types';
import { useDeleteTunnelModal } from './useDeleteTunnelModal';

const { deleteTunnel } = clientApi;

const invalidateOnSuccess = [clientQueryKeys.getTunnels, clientQueryKeys.getConnections];

export const DeleteTunnelModal = () => {
  const { LL } = useI18nContext();
  const navigate = useNavigate();
  const setClientStore = useClientStore((state) => state.setState);
  const [isOpen, tunnel] = useDeleteTunnelModal(
    (state) => [state.isOpen, state.tunnel],
    shallow,
  );
  const [close, reset] = useDeleteTunnelModal(
    (state) => [state.close, state.reset],
    shallow,
  );
  const toaster = useToaster();
  const localLL = LL.modals.deleteTunnel;
  const queryClient = useQueryClient();

  const { mutate, isPending } = useMutation({
    mutationFn: deleteTunnel,
    onSuccess: () => {
      toaster.success(localLL.messages.success());
      invalidateOnSuccess.forEach((key) => {
        queryClient.invalidateQueries({
          queryKey: [key],
          refetchType: 'active',
        });
      });
      reset();
      setClientStore({
        selectedInstance: {
          id: undefined,
          type: WireguardInstanceType.TUNNEL,
        },
      });
      navigate(routes.client.base, { replace: true });
    },
    onError: (e) => {
      toaster.error(localLL.messages.error());
      console.error(e);
    },
  });

  return (
    <ConfirmModal
      id="delete-tunnel-modal"
      title={localLL.title()}
      subTitle={localLL.subtitle({ name: tunnel?.name ?? '' })}
      type={ConfirmModalType.WARNING}
      isOpen={isOpen && !isUndefined(tunnel)}
      onClose={() => close()}
      afterClose={() => reset()}
      loading={isPending}
      submitText={localLL.controls.submit()}
      cancelText={LL.common.controls.cancel()}
      onSubmit={() => {
        if (tunnel) {
          mutate(tunnel.id);
        }
      }}
      onCancel={() => close()}
    />
  );
};
