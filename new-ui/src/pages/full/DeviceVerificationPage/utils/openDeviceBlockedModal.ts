import { useConfirmModal } from '../../../../shared/hooks/confirmModal/useConfirmModal';
import { deviceBlockedMessage } from './deviceVerificationErrorMessage';

export const openDeviceBlockedModal = () => {
  useConfirmModal.getState().open({
    title: 'Device blocked',
    content: deviceBlockedMessage,
    cancelProps: { text: 'Close' },
  });
};
