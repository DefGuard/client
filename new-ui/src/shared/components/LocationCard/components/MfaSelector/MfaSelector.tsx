import './style.scss';
import clsx from 'clsx';
import { type HTMLProps, type MouseEventHandler, useMemo } from 'react';
import { MfaMethod, type MfaMethodValue } from '../../../../rust-api/types';
import { mfaToText } from '../../../../utils/mfa';
import { Icon, type IconKindValue } from '../../../Icon';
import checkboxSrc from './assets/checkbox.svg';

interface Props {
  factor: MfaMethodValue;
  selected?: boolean;
  isDefault?: boolean;
  configured?: boolean;
  isSelectable: boolean;
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
  isSelectable,
}: Props) => {
  const iconKind = useMemo((): IconKindValue => {
    switch (factor) {
      case 'email':
        return 'mail';
      case 'mobileapprove':
        return 'qr';
      case 'oidc':
        return 'token';
      case 'totp':
        return 'lock-closed';
      case 'biometric':
        return 'biometric';
      case 'fido2':
        return 'software-key';
    }
  }, [factor]);

  const isMobileOnly = factor === MfaMethod.Biometric;
  const showCheckbox = isSelectable && selected;

  return (
    <div
      {...containerProps}
      aria-disabled={!isSelectable}
      data-factor={factor}
      className={clsx(containerProps?.className, 'mfa-selector', {
        selected,
        disabled: !isSelectable,
      })}
      onClick={(event) => {
        if (isSelectable) {
          onClick?.(event);
        }
      }}
    >
      {showCheckbox && <img src={checkboxSrc} alt="" width={24} height={24} />}
      {!showCheckbox && (
        <div className="icon-col">
          <Icon className="factor-icon" icon={iconKind} size={20} />
        </div>
      )}
      <div className="middle">
        <p className="name">{mfaToText(factor)}</p>
      </div>
      <div className="right">
        {(isMobileOnly || !configured) && (
          <p className="disabled-label">
            {isMobileOnly ? 'Mobile client only' : 'Not configured'}
          </p>
        )}
        {isSelectable && configured && isDefault && (
          <div className="default-badge">
            <p>Default</p>
          </div>
        )}
      </div>
    </div>
  );
};
