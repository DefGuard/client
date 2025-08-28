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
import { EnrollmentStepIndicator } from '../../components/EnrollmentStepIndicator/EnrollmentStepIndicator';
import { EnrollmentStepKey } from '../../const';
import { EnrollmentNavDirection } from '../../hooks/types';

const formSchema = z.object({
  code: z.string().trim().min(6, 'Enter valid code').max(6, 'Enter valid code'),
});

type FormFields = z.infer<typeof formSchema>;

export const MfaSetupStep = () => {
  const toaster = useToaster();
  const submitRef = useRef<HTMLInputElement>(null);
  const [userInfo, mfaMethod] = useEnrollmentStore((s) => [s.userInfo, s.mfaMethod]);
  const [nextSubject, setStoreState] = useEnrollmentStore(
    (s) => [s.nextSubject, s.setState],
    shallow,
  );

  const {
    enrollment: { registerCodeMfaFinish, registerCodeMfaStart },
  } = useEnrollmentApi();

  const { data: startData, isLoading: startLoading } = useQuery({
    queryFn: () => registerCodeMfaStart(mfaMethod),
    queryKey: ['register-mfa', mfaMethod],
    refetchOnWindowFocus: false,
    enabled: isPresent(mfaMethod),
  });

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

  const isLoading = startLoading || isPending;

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
      loading: startLoading || isPending,
    });
  }, [startLoading, isPending]);

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
      {isLoading && (
        <div className="loading">
          <LoaderSpinner size={64} />
        </div>
      )}
      {!isLoading && isPresent(userInfo) && (
        <>
          {mfaMethod === MfaMethod.TOTP && (
            <>
              <MessageBox
                message={
                  'To setup your MFA, scan this QR code with your authenticator app, then enter the code in the field below:'
                }
              />
              {isPresent(startData?.totp_secret) && (
                <TotpQr email={userInfo.email} secret={startData.totp_secret} />
              )}
            </>
          )}
          {mfaMethod === MfaMethod.EMAIL && (
            <MessageBox>
              <p>
                To setup your MFA, enter the code that was sent to your account email:
                <br />
                <strong>{userInfo.email}</strong>
              </p>
            </MessageBox>
          )}
          <form onSubmit={handleSubmit(submitHandler)}>
            <FormInput
              controller={{ control, name: 'code' }}
              label={mfaMethod === MfaMethod.TOTP ? 'Authenticator code' : 'Email code'}
            />
            <input type="submit" ref={submitRef} />
          </form>
        </>
      )}
    </Card>
  );
};

type TotpProps = {
  email: string;
  secret: string;
};

const TotpQr = ({ email, secret }: TotpProps) => {
  const { writeToClipboard } = useClipboard();
  return (
    <div className="totp-info">
      <div className="qr">
        <QRCode value={`otpauth://totp/Defguard:${email}?secret=${secret}`} />
      </div>
      <Button
        text="Copy TOTP"
        icon={<SvgIconCopy />}
        onClick={() => {
          writeToClipboard(secret);
        }}
      />
    </div>
  );
};
