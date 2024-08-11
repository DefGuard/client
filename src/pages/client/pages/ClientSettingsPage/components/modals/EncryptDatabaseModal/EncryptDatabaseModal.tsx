import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useToaster } from '../../../../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../../../clientAPI/clientApi';
import { clientQueryKeys } from '../../../../../query';
import { useEncryptDatabaseModal } from './hooks/useEncryptDatabaseModal';

const passwordExp = /^[A-Za-z0-9]+$/;

const { protectDatabase } = clientApi;

export const EncryptDatabaseModal = () => {
  const isOpen = useEncryptDatabaseModal((s) => s.visible);
  const [close, reset] = useEncryptDatabaseModal((s) => [s.close, s.reset], shallow);

  return (
    <ModalWithTitle
      title="Encrypt Database"
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const toaster = useToaster();
  const queryClient = useQueryClient();
  const { LL } = useI18nContext();
  const closeModal = useEncryptDatabaseModal((s) => s.close);
  const schema = useMemo(
    () =>
      z
        .object({
          password: z
            .string()
            .min(1, LL.form.errors.required())
            .min(8, LL.form.errors.minLength({ length: 8 }))
            .regex(passwordExp, LL.form.errors.invalid()),
          repeat: z.string().min(1, LL.form.errors.required()),
        })
        .superRefine(({ password, repeat }, ctx) => {
          if (password !== repeat) {
            ctx.addIssue({
              code: 'custom',
              message: "Doesn't match given password.",
              path: ['repeat'],
            });
          }
        }),
    [LL.form.errors],
  );

  type FormFields = z.infer<typeof schema>;

  const {
    handleSubmit,
    control,
    formState: { isSubmitting },
  } = useForm<FormFields>({
    defaultValues: {
      password: '',
      repeat: '',
    },
    resolver: zodResolver(schema),
    mode: 'all',
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (vals): Promise<void> => {
    await protectDatabase(vals.password.trim()).then(() => {
      toaster.success('Database Encrypted.');
      queryClient.invalidateQueries({
        queryKey: [clientQueryKeys.getAppConfig],
      });
      closeModal();
    });
  };

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput
        type="password"
        autoComplete="off"
        controller={{ control, name: 'password' }}
        label="Password"
      />
      <FormInput
        type="password"
        autoComplete="off"
        controller={{ control, name: 'repeat' }}
        label="Confirm password"
      />
      <div className="controls">
        <Button
          text={LL.common.controls.cancel()}
          type="button"
          onClick={() => closeModal()}
          disabled={isSubmitting}
          styleVariant={ButtonStyleVariant.STANDARD}
        />
        <Button
          text={LL.common.controls.submit()}
          type="submit"
          loading={isSubmitting}
          styleVariant={ButtonStyleVariant.PRIMARY}
        />
      </div>
    </form>
  );
};
