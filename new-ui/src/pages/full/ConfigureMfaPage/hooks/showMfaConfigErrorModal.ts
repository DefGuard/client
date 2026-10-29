import { ButtonVariant } from '../../../../shared/components/Button/types';
import { useConfirmModal } from '../../../../shared/hooks/confirmModal/useConfirmModal';

export const showMfaConfigErrorModal = (message: string): (() => void) => {
  const cancelProps = { text: 'Got it', variant: ButtonVariant.Primary };
  useConfirmModal.getState().open({
    title: 'MFA configuration error',
    content: message,
    cancelProps,
  });

  return () => {
    // a later caller may have replaced this modal before the step unmounts
    if (useConfirmModal.getState().cancelProps === cancelProps) {
      useConfirmModal.setState({ visible: false });
    }
  };
};
