import './style.scss';
import clsx from 'clsx';
import { type HTMLProps, type MouseEventHandler, useMemo } from 'react';
import { MfaMethod, type MfaMethodValue } from '../../../../rust-api/types';
import { mfaToText } from '../../../../utils/mfa';
import { Icon, IconKind, type IconKindValue } from '../../../Icon';

interface Props {
  factor: MfaMethodValue;
  selected?: boolean;
  isDefault?: boolean;
  configured?: boolean;
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

  const isMobileOnly = factor === MfaMethod.Biometric;
  const selectable = configured && !isMobileOnly;

  return (
    <div
      {...containerProps}
      className={clsx(containerProps?.className, 'mfa-selector', {
        selected,
        disabled: !selectable,
      })}
      aria-disabled={!selectable}
      onClick={(event) => {
        if (selectable) {
          onClick?.(event);
        }
      }}
      data-factor={factor}
    >
      <Icon className="factor-icon" icon={iconKind} size={20} />
      <div className="middle">
        <p className="name">{mfaToText(factor)}</p>
      </div>
      <div className="right">
        {!selectable && (
          <p className="disabled-label">
            {isMobileOnly ? 'Mobile client only' : 'Not configured'}
          </p>
        )}
        {selectable && isDefault && (
          <div className="default-badge">
            <p>Default</p>
          </div>
        )}
        {selectable && !isDefault && selected && <Icon icon={IconKind.Check} size={16} />}
      </div>
    </div>
  );
};
