import { useMutation } from '@tanstack/react-query';
import { useEffect, useMemo, useRef } from 'react';
import { SubmitHandler, useForm } from 'react-hook-form';
import { debounceTime, Subject } from 'rxjs';
import { z } from 'zod';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { FormInput } from '../../../../../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { useClientStore } from '../../../../../../hooks/useClientStore';

export const AppConfigPeerAlive = () => {
  const { LL } = useI18nContext();
  const submitRef = useRef<HTMLInputElement | null>(null);
  const submitSubject = useRef(new Subject<void>());
  const peerAlive = useClientStore((s) => s.appConfig.peer_alive_period);
  const updateAppConfig = useClientStore((s) => s.updateAppConfig);

  const { mutateAsync } = useMutation({
    mutationFn: updateAppConfig,
  });

  const schema = useMemo(
    () =>
      z.object({
        peer_alive: z
          .number({
            required_error: LL.form.errors.required(),
          })
          .min(
            0,
            LL.form.errors.minValue({
              min: 0,
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
    formState: { isValid, isValidating, isDirty },
  } = useForm<FormFields>({
    defaultValues: {
      peer_alive: peerAlive,
    },
  });

  const onValidSubmit: SubmitHandler<FormFields> = async (values) => {
    await mutateAsync({ peer_alive_period: values.peer_alive });
  };

  const watchedFormData = watch('peer_alive');

  useEffect(() => {
    if (isValid && !isValidating && isDirty) {
      submitSubject.current.next();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [watchedFormData, isValidating, isDirty]);

  useEffect(() => {
    const sub = submitSubject.current.pipe(debounceTime(200)).subscribe(() => {
      submitRef.current?.click();
    });
    return () => {
      sub.unsubscribe();
    };
  }, []);

  return (
    <form onSubmit={handleSubmit(onValidSubmit)}>
      <FormInput controller={{ control, name: 'peer_alive' }} />
      <input type="submit" ref={submitRef} className="hidden" />
    </form>
  );
};
