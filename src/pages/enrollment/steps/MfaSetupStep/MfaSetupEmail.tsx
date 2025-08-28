import dayjs, { type Dayjs } from 'dayjs';
import { type ReactNode, useCallback, useEffect, useState } from 'react';
import { interval } from 'rxjs';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import {
  ButtonSize,
  ButtonStyleVariant,
} from '../../../../shared/defguard-ui/components/Layout/Button/types';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import { useToaster } from '../../../../shared/defguard-ui/hooks/toasts/useToaster';
import { useEnrollmentStore } from '../../hooks/store/useEnrollmentStore';

type Props = {
  children: ReactNode;
  userEmail: string;
  refetch: () => void;
};

export const MfaSetupEmail = ({ children, userEmail, refetch }: Props) => {
  const toaster = useToaster();
  const checkResend = useCallback((resend: Dayjs | undefined) => {
    if (resend) {
      const cd = resend.add(1, 'minute');
      return cd.isBefore(dayjs());
    }
    return true;
  }, []);

  const resendTimestamp = useEnrollmentStore((s) => s.emailResendTimestamp);
  const setStore = useEnrollmentStore((s) => s.setState);

  const [canResent, setCanResend] = useState(checkResend(resendTimestamp));

  useEffect(() => {
    const sub = interval(1000).subscribe(() => {
      if (resendTimestamp) {
        setCanResend(checkResend(resendTimestamp));
      }
    });
    return () => {
      sub.unsubscribe();
    };
  }, [resendTimestamp, checkResend]);

  return (
    <div className="email-setup setup-container">
      <MessageBox>
        <p>
          To setup your MFA, enter the code that was sent to your account email:
          <br />
          <strong>{userEmail}</strong>
        </p>
      </MessageBox>
      {children}
      <Button
        className="resend"
        disabled={!canResent}
        styleVariant={ButtonStyleVariant.STANDARD}
        size={ButtonSize.LARGE}
        text="Resend Email"
        onClick={() => {
          setStore({
            emailResendTimestamp: dayjs(),
          });
          refetch();
          toaster.success('Email resent');
        }}
      />
    </div>
  );
};
