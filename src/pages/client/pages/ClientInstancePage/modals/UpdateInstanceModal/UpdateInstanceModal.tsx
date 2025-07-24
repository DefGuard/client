import './style.scss';

import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import useEffectOnce from '../../../../../../shared/defguard-ui/utils/useEffectOnce';
import { useDeleteInstanceModal } from '../DeleteInstanceModal/useDeleteInstanceModal';
import { UpdateInstanceModalForm } from './components/UpdateInstanceModalForm';
import { useUpdateInstanceModal } from './useUpdateInstanceModal';

export const UpdateInstanceModal = () => {
  const { LL } = useI18nContext();
  const localLL = LL.modals.updateInstance;
  const isOpen = useUpdateInstanceModal((state) => state.isOpen);
  const [close, reset] = useUpdateInstanceModal(
    (state) => [state.close, state.reset],
    shallow,
  );
  const isDeleteOpen = useDeleteInstanceModal((state) => state.isOpen);

  useEffectOnce(() => {
    reset();
  });

  return (
    <ModalWithTitle
      id="update-instnace-modal"
      title={localLL.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      disableClose={isDeleteOpen}
    >
      <MessageBox
        type={MessageBoxType.INFO}
        message={localLL.infoMessage()}
        dismissId="update-instance-modal-info"
      />
      <UpdateInstanceModalForm />
    </ModalWithTitle>
  );
};
