import './style.scss';
import clsx from 'clsx';
import { type LocationInfo, MfaMethod } from '../../../../rust-api/types';
import { mfaToText } from '../../../../utils/mfa';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  onEdit: () => void;
}

export const LocationCardMfaEdit = ({ location, onEdit, variant }: Props) => {
  const mfaMethod = location.mfa_method ?? MfaMethod.Totp;

  if (location.location_mfa_mode === 'disabled') return null;

  return (
    <div className={clsx('location-card-mfa-edit', `variant-${variant}`)}>
      <div className="mfa-badge">
        <p>MFA</p>
      </div>
      <p className="name">{mfaToText(mfaMethod)}</p>
      {location.location_mfa_mode === 'internal' && (
        <IconButton
          variant={IconButtonVariant.SmallSelected}
          icon="edit"
          onClick={onEdit}
        />
      )}
    </div>
  );
};
