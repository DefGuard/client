import clsx from 'clsx';
import './style.scss';
import { Icon } from '../../../../../shared/components/Icon';
import { RadioIndicator } from '../../../../../shared/components/RadioIndicator/RadioIndicator';
import { mfaMethodIcon } from '../../../../../shared/consts';
import { MfaMethod, type MfaMethodValue } from '../../../../../shared/rust-api/types';

type FactorText = {
  title: string;
  description: string;
};

const factorText = (method: MfaMethodValue): FactorText => {
  switch (method) {
    case MfaMethod.Totp:
      return {
        title: 'Security code from your authenticator app',
        description: `Use an app on your phone that generates a temporary code to confirm it's you (Google Authenticator, Microsoft Authenticator, Authy).`,
      };
    case MfaMethod.Email:
      return {
        title: `Code from email`,
        description: `We'll send a temporary security code to your email. Enter the code to confirm it's you and continue.`,
      };
    case MfaMethod.Oidc:
      return { title: '', description: '' };
    case MfaMethod.Biometric:
      return { title: '', description: '' };
    case MfaMethod.MobileApprove:
      return { title: '', description: '' };
    case MfaMethod.Fido2:
      return { title: '', description: '' };
  }
};

interface Props {
  mfaMethod: MfaMethodValue;
  selected: boolean;
  disabled?: boolean;
  onClick: () => void;
}

export const ConfigureMfaVerificatorFactorSelector = ({
  mfaMethod,
  selected,
  disabled = false,
  onClick,
}: Props) => {
  const { title, description } = factorText(mfaMethod);

  return (
    <div
      className={clsx('mfa-verification-factor-selector', { selected, disabled })}
      data-method={mfaMethod}
      onClick={() => {
        if (!disabled) onClick();
      }}
    >
      <div className="inner-grid">
        <div className="icon-col">
          <Icon icon={mfaMethodIcon[mfaMethod]} size={20} />
        </div>
        <div className="content-col">
          <p className="title">{title}</p>
          <p className="description">{description}</p>
        </div>
        <div className="indicator-col">
          <RadioIndicator active={selected} />
        </div>
      </div>
    </div>
  );
};
