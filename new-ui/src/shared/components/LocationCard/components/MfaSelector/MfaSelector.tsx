import './style.scss';
import clsx from 'clsx';
import { type HTMLProps, type MouseEventHandler, useMemo } from 'react';
import type { MfaMethod } from '../../../../rust-api/types';
import { mfaToText } from '../../../../utils/mfa';
import { Icon, IconKind, type IconKindValue } from '../../../Icon';

interface Props {
  factor: MfaMethod;
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
}: Props) => {
  const iconKind = useMemo((): IconKindValue => {
    switch (factor) {
      case 'email':
        return 'mail';
      case 'mobileapprove':
        return 'mobile-lock';
      case 'oidc':
        return 'token';
      case 'totp':
        return 'mobile-lock';
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
      <Icon className="factor-icon" icon={iconKind} size={20} />
      <div className="middle">
        <p className="name">{mfaToText(factor)}</p>
        {isDefault && (
          <div className="default-badge">
            <p>Default</p>
          </div>
        )}
      </div>
      {selected && <Icon icon={IconKind.Check} size={16} />}
    </div>
  );
};
