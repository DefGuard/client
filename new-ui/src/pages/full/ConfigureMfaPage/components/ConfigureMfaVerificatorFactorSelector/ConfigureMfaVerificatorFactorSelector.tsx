import clsx from 'clsx';
import './style.scss';
import { Icon } from '../../../../../shared/components/Icon';
import { RadioIndicator } from '../../../../../shared/components/RadioIndicator/RadioIndicator';
import { mfaMethodIcon } from '../../../../../shared/consts';
import { MfaMethod, type MfaMethodValue } from '../../../../../shared/rust-api/types';
import {
  mfaToText,
  type OpenIdInstance,
  openIdProviderName,
} from '../../../../../shared/utils/mfa';

type FactorText = {
  title: string;
  description: string;
};

const factorText = (method: MfaMethodValue, instance?: OpenIdInstance): FactorText => {
  switch (method) {
    case MfaMethod.Totp:
      return {
        title: 'Security code from your authenticator app',
        description: `Use an authenticator app on your phone, such as Google Authenticator, Microsoft Authenticator, or Authy, to generate a temporary code and verify your identity.`,
      };
    case MfaMethod.Email:
      return {
        title: `Code from email`,
        description: `Enter the temporary security code sent to your email to verify your identity and continue.`,
      };
    case MfaMethod.Oidc:
      return {
        title: `Verify with ${openIdProviderName(instance)}`,
        description: `Sign in through your identity provider in the browser to verify your identity.`,
      };
    case MfaMethod.Biometric:
      return { title: '', description: '' };
    case MfaMethod.MobileApprove:
      return { title: '', description: '' };
    case MfaMethod.Fido2:
      return {
        title: mfaToText(MfaMethod.Fido2),
        description: `Verify your identity using a registered security key, such as a YubiKey.`,
      };
  }
};

interface Props {
  mfaMethod: MfaMethodValue;
  instance?: OpenIdInstance;
  selected: boolean;
  disabled?: boolean;
  onClick: () => void;
}

export const ConfigureMfaVerificatorFactorSelector = ({
  mfaMethod,
  instance,
  selected,
  disabled = false,
  onClick,
}: Props) => {
  const { title, description } = factorText(mfaMethod, instance);

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
