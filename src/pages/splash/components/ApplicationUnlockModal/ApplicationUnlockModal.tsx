import { zodResolver } from '@hookform/resolvers/zod';
import { useQueryClient } from '@tanstack/react-query';
import { useMemo } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { ButtonStyleVariant } from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { ModalWithTitle } from '../../../../shared/defguard-ui/components/Layout/modals/ModalWithTitle/ModalWithTitle';
import { useToaster } from '../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { clientApi } from '../../../client/clientAPI/clientApi';
import { clientQueryKeys } from '../../../client/query';
import { useApplicationUnlockModal } from './useApplicationUnlockModal';

const { unlockDatabase } = clientApi;

export const ApplicationUnlockModal = () => {
  const isOpen = useApplicationUnlockModal((s) => s.visible);
  const [close, reset] = useApplicationUnlockModal((s) => [s.close, s.reset], shallow);

  return (
    <ModalWithTitle
      id="application-unlock-modal"
      title="Enter Application Password"
      isOpen={isOpen}
      onClose={close}
      afterClose={reset}
      disableClose={true}
    >
      <ModalContent />
    </ModalWithTitle>
  );
};

const ModalContent = () => {
  const { LL } = useI18nContext();
  const queryClient = useQueryClient();
  const toaster = useToaster();
  const schema = useMemo(
    () =>
      z
        .object({
          password: z.string().min(8, LL.form.errors.minLength({ length: 8 })),
        })
        .required(),
    [LL],
  );

  type FormFields = z.infer<typeof schema>;

  const {
    handleSubmit,
    control,
    formState: { isSubmitting },
    setError,
  } = useForm<FormFields>({
    mode: 'all',
    defaultValues: {
      password: '',
    },
    resolver: zodResolver(schema),
  });

  const onValid: SubmitHandler<FormFields> = async (values) => {
    try {
      await unlockDatabase(values.password);
      toaster.success('Application Unlocked');
      queryClient.invalidateQueries({ queryKey: [clientQueryKeys.getAppConfig] });
      queryClient.invalidateQueries({
        queryKey: [clientQueryKeys.getAppDatabaseConnectionStatus],
      });
    } catch (e) {
      toaster.error('Failed to unlock database');
      setError(
        'password',
        {
          message: 'Wrong password',
        },
        {
          shouldFocus: true,
        },
      );
      console.error(e);
    }
  };

  return (
    <form onSubmit={handleSubmit(onValid)}>
      <FormInput
        label="Password"
        placeholder="Application password"
        controller={{ control, name: 'password' }}
        type="password"
        autoComplete="password"
      />
      <div className="controls">
        <Button
          text={LL.common.controls.submit()}
          type="submit"
          styleVariant={ButtonStyleVariant.PRIMARY}
          loading={isSubmitting}
        />
      </div>
    </form>
  );
};
