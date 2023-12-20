import './style.scss';

import { useEffect } from 'react';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { MessageBox } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { ModalWithTitle } from '../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
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

  // reset state on page mount
  useEffect(() => {
    reset();
    // eslint-disable-next-line
  }, []);

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
