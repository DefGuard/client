import './style.scss';

import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery } from '@tanstack/react-query';
import { error } from '@tauri-apps/plugin-log';
import { type Ref, useEffect, useRef } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import z from 'zod';
import { shallow } from 'zustand/shallow';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { useToaster } from '../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { MfaMethod } from '../../../../shared/types';
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { EnrollmentStepKey } from '../../const';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { EnrollmentNavDirection } from '../../hooks/types';
import { useEnrollmentApi } from '../../hooks/useEnrollmentApi';
import { MfaSetupEmail } from './MfaSetupEmail';
import { MfaSetupTotp } from './MfaSetupTotp';

const formSchema = z.object({
  code: z.string().trim().min(6, 'Enter valid code').max(6, 'Enter valid code'),
});

type FormFields = z.infer<typeof formSchema>;

export const MfaSetupStep = () => {
  const submitRef = useRef<HTMLInputElement>(null);
  const [userInfo, mfaMethod] = useEnrollmentStore((s) => [s.userInfo, s.mfaMethod]);
  const [nextSubject, setStoreState] = useEnrollmentStore(
    (s) => [s.nextSubject, s.setState],
    shallow,
  );

  const {
    enrollment: { registerCodeMfaStart },
  } = useEnrollmentApi();

  const {
    data: startData,
    isLoading: startLoading,
    refetch,
  } = useQuery({
    queryFn: () => registerCodeMfaStart(mfaMethod),
    queryKey: ['register-mfa', mfaMethod],
    refetchOnWindowFocus: false,
    enabled: isPresent(mfaMethod),
  });

  // biome-ignore lint/correctness/useExhaustiveDependencies: rxjs sub
  useEffect(() => {
    const sub = nextSubject.subscribe((direction) => {
      switch (direction) {
        case EnrollmentNavDirection.NEXT:
          submitRef.current?.click();
          break;
        case EnrollmentNavDirection.BACK:
          setStoreState({ step: EnrollmentStepKey.MFA_CHOICE });
          break;
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject]);

  return (
    <Card id="enrollment-totp-card">
      <div>
        <EnrollmentStepIndicator />
        <h3>Configure MFA</h3>
      </div>
      {startLoading && (
        <div className="loading">
          <LoaderSpinner size={64} />
        </div>
      )}
      {!startLoading && isPresent(userInfo) && (
        <>
          {mfaMethod === MfaMethod.TOTP && isPresent(startData?.totp_secret) && (
            <MfaSetupTotp email={userInfo.email} secret={startData.totp_secret}>
              <CodeForm inputRef={submitRef} />
            </MfaSetupTotp>
          )}
          {mfaMethod === MfaMethod.EMAIL && (
            <MfaSetupEmail refetch={refetch} userEmail={userInfo.email}>
              <CodeForm inputRef={submitRef} />
            </MfaSetupEmail>
          )}
        </>
      )}
    </Card>
  );
};

type CodeFormProps = {
  inputRef: Ref<HTMLInputElement>;
};

const CodeForm = ({ inputRef }: CodeFormProps) => {
  const toaster = useToaster();
  const mfaMethod = useEnrollmentStore((s) => s.mfaMethod);
  const setStoreState = useEnrollmentStore((s) => s.setState);
  const {
    enrollment: { registerCodeMfaFinish },
  } = useEnrollmentApi();

  const { handleSubmit, control, setError } = useForm<FormFields>({
    resolver: zodResolver(formSchema),
  });

  const { mutate, isPending } = useMutation({
    mutationFn: registerCodeMfaFinish,
    onSuccess: (response) => {
      toaster.success('MFA configured');
      setStoreState({
        loading: false,
        step: EnrollmentStepKey.MFA_RECOVERY,
        recoveryCodes: response.recovery_codes,
      });
    },
    onError: (err) => {
      setError(
        'code',
        {
          message: 'Enter valid code',
          type: 'value',
        },
        {
          shouldFocus: false,
        },
      );
      error(`MFA configuration failed! \nReason: ${err.message}`);
      console.error(err);
    },
  });

  const submitHandler: SubmitHandler<FormFields> = (data) => {
    const sendData = {
      code: data.code,
      method: mfaMethod,
    };
    mutate(sendData);
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: sideEffect
  useEffect(() => {
    setStoreState({
      loading: isPending,
    });
  }, [isPending]);

  return (
    <form onSubmit={handleSubmit(submitHandler)}>
      <FormInput
        controller={{ control, name: 'code' }}
        label={mfaMethod === MfaMethod.TOTP ? 'Authenticator code' : 'Email code'}
      />
      <input type="submit" ref={inputRef} />
    </form>
  );
};
