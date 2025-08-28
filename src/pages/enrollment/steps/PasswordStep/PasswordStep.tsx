import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useEffect, useMemo, useRef } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { passwordValidator } from '../../../../shared/validators/password';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { EnrollmentStepKey } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { EnrollmentNavDirection } from '../../hooks/types';

type FormFields = {
  password: string;
  repeat: string;
};

export const PasswordStep = () => {
  const submitRef = useRef<HTMLInputElement | null>(null);
  const { LL } = useI18nContext();

  const userPassword = useEnrollmentStore((state) => state.userPassword);
  const [nextSubject, setStore] = useEnrollmentStore(
    (state) => [state.nextSubject, state.setState],
    shallow,
  );

  const pageLL = LL.pages.enrollment.steps.password;

  const schema = useMemo(
    () =>
      z
        .object({
          password: passwordValidator(LL),
          repeat: z.string().min(1, LL.form.errors.required()),
        })
        .superRefine((values, ctx) => {
          if (values.password !== values.repeat && values.repeat.length >= 1) {
            ctx.addIssue({
              path: ['repeat'],
              message: pageLL.form.fields.repeat.errors.matching(),
              code: 'custom',
            });
          }
        }),
    [LL, pageLL.form.fields.repeat.errors],
  );

  const { handleSubmit, control, reset } = useForm<FormFields>({
    defaultValues: {
      password: userPassword ?? '',
      repeat: '',
    },
    mode: 'all',
    criteriaMode: 'all',
    resolver: zodResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    setStore({ userPassword: values.password, step: EnrollmentStepKey.DEVICE });
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: rxjs
  useEffect(() => {
    const sub = nextSubject.subscribe((direction) => {
      switch (direction) {
        case EnrollmentNavDirection.BACK:
          setStore({ step: EnrollmentStepKey.DATA_VERIFICATION });
          break;
        case EnrollmentNavDirection.NEXT:
          submitRef.current?.click();
          break;
      }
    });

    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject]);

  useEffect(() => {
    reset();
    //eslint-disable-next-line
  }, [reset]);

  return (
    <Card id="enrollment-password-card">
      <EnrollmentStepIndicator />
      <h3>{pageLL.title()}</h3>
      <form
        data-testid="enrollment-password-form"
        onSubmit={handleSubmit(handleValidSubmit)}
      >
        <FormInput
          label={pageLL.form.fields.password.label()}
          controller={{
            control,
            name: 'password',
            rules: {
              deps: ['repeat'],
            },
          }}
          type="password"
          floatingErrors={{
            title: LL.form.errors.password.floatingTitle(),
          }}
          autoComplete="new-password"
        />
        <FormInput
          label={pageLL.form.fields.repeat.label()}
          controller={{
            control,
            name: 'repeat',
            rules: {
              deps: ['password'],
            },
          }}
          type="password"
          autoComplete="new-password"
        />
        <input className="hidden" type="submit" ref={submitRef} />
      </form>
    </Card>
  );
};
