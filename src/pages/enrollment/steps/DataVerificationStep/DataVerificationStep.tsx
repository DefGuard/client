import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { z } from 'zod';
import { shallow } from 'zustand/shallow';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { MessageBoxType } from '../../../../shared/defguard-ui/components/Layout/MessageBox/types';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

const phonePattern = /^\+?[0-9]+( [0-9]+)?$/;

type FormFields = {
  phone: string;
};

export const DataVerificationStep = () => {
  const { LL } = useI18nContext();
  const submitRef = useRef<HTMLInputElement | null>(null);

  const nextSubject = useEnrollmentStore((state) => state.nextSubject);

  const userInfo = useEnrollmentStore((state) => state.userInfo);

  const [setEnrollment, next] = useEnrollmentStore(
    (state) => [state.setState, state.nextStep],
    shallow,
  );

  const pageLL = LL.pages.enrollment.steps.dataVerification;

  const schema = useMemo(
    () =>
      z.object({
        phone: z
          .string()
          .trim()
          .nonempty(LL.form.errors.required())
          .regex(phonePattern, LL.form.errors.invalid()),
      }),
    [LL.form.errors],
  );

  const { control, handleSubmit } = useForm<FormFields>({
    defaultValues: {
      phone: userInfo?.phone_number ?? '',
    },
    mode: 'all',
    resolver: zodResolver(schema),
  });

  const handleValidSubmit: SubmitHandler<FormFields> = (values) => {
    if (userInfo) {
      setEnrollment({
        userInfo: { ...userInfo, phone_number: values.phone },
      });
      next();
    }
  };

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      submitRef.current?.click();
    });

    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject]);

  return (
    <Card id="enrollment-data-verification-card">
      <EnrollmentStepIndicator />
      <h3>{pageLL.title()}</h3>
      <MessageBox type={MessageBoxType.INFO} message={pageLL.messageBox()} />
      <form
        data-testid="enrollment-data-verification"
        onSubmit={handleSubmit(handleValidSubmit)}
      >
        <div className="row">
          <div className="item">
            <label>{pageLL.form.fields.firstName.label()}:</label>
            <p>{userInfo?.first_name}</p>
          </div>
          <div className="item">
            <label>{pageLL.form.fields.lastName.label()}:</label>
            <p>{userInfo?.last_name}</p>
          </div>
        </div>
        <div className="row">
          <div className="item">
            <label>{pageLL.form.fields.email.label()}:</label>
            <p>{userInfo?.email}</p>
          </div>
          <FormInput
            label={pageLL.form.fields.phone.label()}
            controller={{ control, name: 'phone' }}
          />
        </div>
        <input className="hidden" ref={submitRef} type="submit" />
      </form>
    </Card>
  );
};
