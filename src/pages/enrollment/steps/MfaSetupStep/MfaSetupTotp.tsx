import type { ReactNode } from 'react';
import QRCode from 'react-qr-code';
import { Button } from '../../../../shared/defguard-ui/components/Layout/Button/Button';
import { MessageBox } from '../../../../shared/defguard-ui/components/Layout/MessageBox/MessageBox';
import SvgIconCopy from '../../../../shared/defguard-ui/components/svg/IconCopy';
import { useClipboard } from '../../../../shared/hooks/useClipboard';

type Props = {
  children: ReactNode;
  email: string;
  secret: string;
};

export const MfaSetupTotp = ({ children, email, secret }: Props) => {
  return (
    <div className="totp-setup setup-container">
      <MessageBox
        message={
          'To setup your MFA, scan this QR code with your authenticator app, then enter the code in the field below:'
        }
      />
      <TotpQr email={email} secret={secret} />
      {children}
    </div>
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
