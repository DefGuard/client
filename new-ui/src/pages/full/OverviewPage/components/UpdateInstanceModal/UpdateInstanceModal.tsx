import { useEffect, useMemo, useState } from 'react';
import z from 'zod';
import { Button } from '../../../../../shared/components/Button/Button';
import { ButtonVariant } from '../../../../../shared/components/Button/types';
import { Controls } from '../../../../../shared/components/Controls/Controls';
import { Modal } from '../../../../../shared/components/Modal/Modal';
import { SizedBox } from '../../../../../shared/components/SizedBox/SizedBox';
import { edgeApi } from '../../../../../shared/edge-api/api';
import { useAppForm } from '../../../../../shared/form';
import { formChangeLogic } from '../../../../../shared/formLogic';
import {
  closeModal,
  subscribeCloseModal,
  subscribeOpenModal,
} from '../../../../../shared/hooks/modalControls/modalsSubjects';
import { ModalName } from '../../../../../shared/hooks/modalControls/modalTypes';
import type { OpenUpdateInstanceModalData } from '../../../../../shared/hooks/modalControls/types';
import { Snackbar } from '../../../../../shared/providers/snackbar/snackbar';
import { ThemeSpacing } from '../../../../../shared/types';
import { isPresent } from '../../../../../shared/utils/isPresent';

const modalNameKey = ModalName.UpdateInstance;

export const UpdateInstanceModal = () => {
  const [isOpen, setOpen] = useState(false);
  const [modalData, setModalData] = useState<OpenUpdateInstanceModalData | null>(null);

  useEffect(() => {
    const openSub = subscribeOpenModal(modalNameKey, (data) => {
      setModalData(data);
      setOpen(true);
    });
    const closeSub = subscribeCloseModal(modalNameKey, () => setOpen(false));
    return () => {
      openSub.unsubscribe();
      closeSub.unsubscribe();
    };
  }, []);
  return (
    <Modal
      size="small"
      title="Update instance"
      id="update-instance-modal"
      isOpen={isOpen}
      onClose={() => {
        setOpen(false);
      }}
    >
      {isOpen && isPresent(modalData) && <ModalContent data={modalData} />}
    </Modal>
  );
};

const formSchema = z.object({
  url: z.string().min(1, 'Field is required'),
  token: z.string('Field is required').min(1, 'Field is required.'),
});

type FormFields = z.infer<typeof formSchema>;

const ModalContent = ({ data }: { data: OpenUpdateInstanceModalData }) => {
  const defaultValues = useMemo(
    (): FormFields => ({
      url: data.url,
      token: '',
    }),
    [data.url],
  );

  const form = useAppForm({
    defaultValues,
    validationLogic: formChangeLogic,
    validators: {
      onSubmit: formSchema,
      onChange: formSchema,
    },
    onSubmit: async ({ value, formApi }) => {
      const result = await edgeApi.updateInstance({
        instanceId: data.instanceId,
        url: value.url,
        token: value.token,
      });
      if (result.error) {
        if (result.isCredentialsError) {
          formApi.setErrorMap({
            onSubmit: {
              fields: {
                token: 'Invalid Token or URL',
                url: 'Invalid Token or URL',
              },
            },
          });
        } else {
          Snackbar.error(result.error);
        }
        return;
      }
      Snackbar.default('Instance updated.');
      closeModal(modalNameKey);
    },
  });

  return (
    <form
      onSubmit={(e) => {
        e.stopPropagation();
        e.preventDefault();
        form.handleSubmit();
      }}
    >
      <form.AppForm>
        <form.AppField name="url">
          {(field) => <field.FormInput label="Instance URL" required />}
        </form.AppField>
        <SizedBox height={ThemeSpacing.Xl} />
        <form.AppField name="token">
          {(field) => <field.FormInput label="Token" required />}
        </form.AppField>
        <Controls>
          <div className="right">
            <Button
              variant={ButtonVariant.Secondary}
              text="Cancel"
              onClick={() => {
                closeModal(modalNameKey);
              }}
            />
            <form.Subscribe selector={(s) => s.isSubmitting}>
              {(isSubmitting) => (
                <Button
                  variant={ButtonVariant.Primary}
                  text="Update"
                  loading={isSubmitting}
                  onClick={() => {
                    form.handleSubmit();
                  }}
                />
              )}
            </form.Subscribe>
          </div>
        </Controls>
      </form.AppForm>
    </form>
  );
};
