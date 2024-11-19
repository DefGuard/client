import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { debounceTime, Subject } from 'rxjs';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { useClientStore } from '../../../../../../hooks/useClientStore';

export const AppConfigConnectionVerificationPeriod = () => {
  const submitRef = useRef<HTMLInputElement | null>(null);
  const subjectRef = useRef(new Subject<void>());
  const storeValue = useClientStore((s) => s.appConfig.connection_verification_time);
  const updateAppConfig = useClientStore((s) => s.updateAppConfig);
  const { mutateAsync } = useMutation({
    mutationFn: updateAppConfig,
  });
  const { LL } = useI18nContext();

  const schema = useMemo(
    () =>
      z.object({
        verificationTime: z
          .number({
            required_error: LL.form.errors.required(),
          })
          .min(
            5,
            LL.form.errors.minValue({
              min: 5,
            }),
          ),
      }),
    [LL.form.errors],
  );

  type FormFields = z.infer<typeof schema>;

  const {
    handleSubmit,
    control,
    watch,
    formState: { isValid, isDirty, isValidating },
  } = useForm<FormFields>({
    defaultValues: {
      verificationTime: storeValue,
    },
  });

  const handleValidSubmit: SubmitHandler<FormFields> = async (values) => {
    await mutateAsync({
      connection_verification_time: values.verificationTime,
    });
  };

  const watchedData = watch('verificationTime');

  useEffect(() => {
    if (isValid && isDirty && !isValidating) {
      subjectRef.current.next();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watchedData, isDirty, isValidating]);

  useEffect(() => {
    const sub = subjectRef.current
      .pipe(debounceTime(200))
      .subscribe(() => submitRef.current?.click());
    return () => {
      sub.unsubscribe();
    };
  }, []);

  return (
    <form onSubmit={handleSubmit(handleValidSubmit)}>
      <FormInput controller={{ control, name: 'verificationTime' }} />
      <input type="submit" className="hidden" ref={submitRef} />
    </form>
  );
};
