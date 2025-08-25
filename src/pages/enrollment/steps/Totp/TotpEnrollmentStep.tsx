import { zodResolver } from '@hookform/resolvers/zod';
import { useMutation, useQuery } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { type SubmitHandler, useForm } from 'react-hook-form';
import QRCode from 'react-qr-code';
import z from 'zod';
import { shallow } from 'zustand/shallow';
import { FormInput } from '../../../../shared/defguard-ui/components/Form/FormInput/FormInput';
import { Card } from '../../../../shared/defguard-ui/components/Layout/Card/Card';
import { LoaderSpinner } from '../../../../shared/defguard-ui/components/Layout/LoaderSpinner/LoaderSpinner';
import { isPresent } from '../../../../shared/defguard-ui/utils/isPresent';
import { MfaMethod } from '../../../../shared/types';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';
import { useEnrollmentApi } from '../../hooks/useEnrollmentApi';
import './style.scss';
import { error } from '@tauri-apps/plugin-log';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import SvgIconCopy from '../../../../shared/defguard-ui/components/svg/IconCopy';
import { useToaster } from '../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { useClipboard } from '../../../../shared/hooks/useClipboard';

const formSchema = z.object({
  code: z.string().min(6, 'Enter valid code').max(6, 'Enter valid code'),
});

type FormFields = z.infer<typeof formSchema>;

export const TotpEnrollmentStep = () => {
  const toaster = useToaster();
  const submitRef = useRef<HTMLInputElement>(null);
  const { writeToClipboard } = useClipboard();
  const userInfo = useEnrollmentStore((s) => s.userInfo);
  const [nextSubject, setStoreState, nextStep] = useEnrollmentStore(
    (s) => [s.nextSubject, s.setState, s.nextStep],
    shallow,
  );

  const {
    enrollment: { registerCodeMfaFinish, registerCodeMfaStart },
  } = useEnrollmentApi();

  const { data: startData, isLoading: startLoading } = useQuery({
    queryFn: () => registerCodeMfaStart(MfaMethod.TOTP),
    queryKey: ['enrollment', 'register-mfa', 'start'],
    refetchOnWindowFocus: false,
  });

  const { handleSubmit, control, setError } = useForm<FormFields>({
    resolver: zodResolver(formSchema),
  });

  const { mutate, isPending } = useMutation({
    mutationFn: registerCodeMfaFinish,
    onSuccess: () => {
      toaster.success('MFA configured');
      setStoreState({ loading: false });
      nextStep();
    },
    onError: (err) => {
      setError(
        'code',
        {
          message: 'Enter valid code',
          type: 'value',
        },
        {
          shouldFocus: true,
        },
      );
      error(`MFA configuration failed! \nReason: ${err.message}`);
      console.error(err);
    },
  });

  const isLoading = startLoading || isPending;

  const submitHandler: SubmitHandler<FormFields> = (data) => {
    mutate(data);
  };

  // biome-ignore lint/correctness/useExhaustiveDependencies: sideEffect
  useEffect(() => {
    setStoreState({
      loading: startLoading || isPending,
    });
  }, [startLoading, isPending]);

  useEffect(() => {
    const sub = nextSubject.subscribe(() => {
      submitRef.current?.click();
    });
    return () => {
      sub.unsubscribe();
    };
  }, [nextSubject]);

  return (
    <Card id="enrollment-totp-card">
      {isLoading && (
        <div className="loading">
          <LoaderSpinner size={64} />
        </div>
      )}
      {!isLoading && (
        <>
          <h3>Configure MFA</h3>
          <MessageBox
            message={
              'To setup your MFA, scan this QR code with your authenticator app, then enter the code in the field below:'
            }
          />
          <form onSubmit={handleSubmit(submitHandler)}>
            {!isLoading &&
              isPresent(startData) &&
              isPresent(startData.totp_secret) &&
              isPresent(userInfo) && (
                <>
                  <div className="qr">
                    <QRCode
                      value={`otpauth://totp/defguard:${userInfo?.email}?secret=${startData?.totp_secret}`}
                    />
                  </div>
                  <Button
                    text="Copy TOTP"
                    icon={<SvgIconCopy />}
                    onClick={() => {
                      writeToClipboard(startData.totp_secret as string);
                    }}
                  />
                  <FormInput controller={{ control, name: 'code' }} label="Code" />
                  <input type="submit" ref={submitRef} />
                </>
              )}
          </form>
        </>
      )}
    </Card>
  );
};
