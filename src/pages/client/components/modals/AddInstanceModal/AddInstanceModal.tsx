import { zodResolver } from '@hookform/resolvers/zod';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';

import { useI18nContext } from '../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useAddInstanceModal } from './hooks/useAddInstanceModal';

export const AddInstanceModal = () => {
  const { LL } = useI18nContext();
  const [isOpen, reset, close] = useAddInstanceModal((state) => [
    state.isOpen,
    state.reset,
    state.close,
  ]);

  return (
    <ModalWithTitle
      title={LL.pages.client.modals.addInstanceModal.title()}
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      backdrop
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

type FormFields = {
  token: string;
};

const defaultValues: FormFields = {
  token: '',
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const closeModal = useAddInstanceModal((state) => state.close);
  const schema = useMemo(
    () =>
      z.object({
        token: z.string().trim().nonempty(LL.form.errors.required()),
      }),
    [LL.form.errors],
  );
  const { handleSubmit, control } = useForm<FormFields>({
    resolver: zodResolver(schema),
    defaultValues,
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    console.table(values);
  };

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput
        controller={{ control, name: 'token' }}
        label={LL.pages.client.modals.addInstanceModal.form.fields.token.label()}
      />
      <div className="controls">
        <Button
          className="close"
          onClick={closeModal}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.STANDARD}
          text={LL.common.controls.cancel()}
        />
        <Button
          type="submit"
          className="submit"
          onClick={() => {}}
          size={ButtonSize.LARGE}
          styleVariant={ButtonStyleVariant.PRIMARY}
          text={LL.common.controls.submit()}
        />
      </div>
    </form>
  );
};
