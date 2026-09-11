import './style.scss';
import { useMutation } from '@tanstack/react-query';
import { useShallow } from 'zustand/shallow';
import { useConfirmModal } from '../../hooks/confirmModal/useConfirmModal';
import { isPresent } from '../../utils/isPresent';
import { Button } from '../Button/Button';
import { ButtonVariant } from '../Button/types';
import { Controls } from '../Controls/Controls';
import { Modal } from '../Modal/Modal';
import { RenderMarkdown } from '../RenderMarkdown/RenderMarkdown';

export const ConfirmModal = () => {
  const [isOpen, title] = useConfirmModal(useShallow((s) => [s.visible, s.title]));

  return (
    <Modal
      size="small"
      id="confirm-modal"
      isOpen={isOpen}
      title={title}
      onClose={() => {
        useConfirmModal.setState({ visible: false });
      }}
      afterClose={() => {
        useConfirmModal.getState().reset();
      }}
    >
      <ModalContent />
    </Modal>
  );
};

const ModalContent = () => {
  const content = useConfirmModal((s) => s.content);

  const [cancelProps, submitProps, onSubmit] = useConfirmModal(
    useShallow((s) => [s.cancelProps, s.submitProps, s.onSubmit]),
  );

  const { mutate, isPending } = useMutation({
    mutationFn: onSubmit,
    onSuccess: () => {
      useConfirmModal.setState({
        visible: false,
      });
    },
  });

  return (
    <>
      <RenderMarkdown content={content} />
      <Controls>
        <div className="right">
          <Button
            text="Cancel"
            variant={ButtonVariant.Secondary}
            {...cancelProps}
            onClick={() => {
              useConfirmModal.setState({ visible: false });
            }}
          />
          {isPresent(submitProps) && (
            <Button
              {...submitProps}
              loading={isPending}
              onClick={() => {
                mutate();
              }}
            />
          )}
        </div>
      </Controls>
    </>
  );
};
