import './style.scss';

import { useShallow } from 'zustand/shallow';

import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Modal } from '../../../../../shared/components/Modal/Modal';
import { useEnrollmentErrorModal } from '../../hooks/useEnrollmentErrorModal';

export const EnrollmentErrorModal = () => {
  const [isOpen, title] = useEnrollmentErrorModal(
    useShallow((s) => [s.visible, s.title]),
  );

  return (
    <Modal
      size="small"
      id="enrollment-error-modal"
      isOpen={isOpen}
      title={title}
      onClose={() => {
        useEnrollmentErrorModal.setState({ visible: false });
      }}
      afterClose={() => {
        useEnrollmentErrorModal.getState().reset();
      }}
    >
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  const message = useEnrollmentErrorModal((s) => s.message);

  return (
    <>
      <p className="message">{message}</p>
      <Controls>
        <div className="right">
          <Button
            text="Got it"
            variant={ButtonVariant.Primary}
            onClick={() => {
              useEnrollmentErrorModal.setState({ visible: false });
            }}
          />
        </div>
      </Controls>
    </>
  );
};
