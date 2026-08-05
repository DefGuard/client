import './style.scss';
import clsx from 'clsx';
import { type HTMLProps, type MouseEventHandler, useMemo } from 'react';
import type { MfaMethodValue } from '../../../../rust-api/types';
import { mfaToText } from '../../../../utils/mfa';
import { CheckboxIndicator } from '../../../CheckboxIndicator/CheckboxIndicator';
import { Icon, IconKind, type IconKindValue } from '../../../Icon';

interface Props {
  factor: MfaMethodValue;
  configured?: boolean;
  canConfigure?: boolean;
  selected?: boolean;
  isDefault?: boolean;
  onClick?: MouseEventHandler<HTMLDivElement>;
  containerProps?: Omit<HTMLProps<HTMLDivElement>, 'onClick'>;
}

export const MfaSelector = ({
  factor,
  onClick,
  containerProps,
  selected = false,
  isDefault = false,
  configured = true,
  canConfigure = false,
}: Props) => {
  const iconKind = useMemo((): IconKindValue => {
    switch (factor) {
      case 'email':
        return 'mail';
      case 'mobileapprove':
        return 'mobile';
      case 'oidc':
        return 'token';
      case 'totp':
        return 'lock-closed';
      case 'biometric':
        return 'biometric';
    }
  }, [factor]);

  return (
    <div
      {...containerProps}
      className={clsx(containerProps?.className, 'mfa-selector', {
        selected,
      })}
      onClick={onClick}
      data-factor={factor}
    >
      <div
        className={clsx('icon-track', {
          configure: canConfigure,
          active: configured,
        })}
      >
        <Icon className="factor-icon" icon={iconKind} size={20} />
        <CheckboxIndicator active={configured} />
      </div>
      <div className="middle">
        <p className="name">{mfaToText(factor)}</p>
        {isDefault && configured && (
          <div className="default-badge">
            <p>Default</p>
          </div>
        )}
      </div>
      {selected && configured && <Icon icon={IconKind.Check} size={16} />}
      {!configured && canConfigure && (
        <p className="configure-label">Select to configure</p>
      )}
    </div>
  );
};
