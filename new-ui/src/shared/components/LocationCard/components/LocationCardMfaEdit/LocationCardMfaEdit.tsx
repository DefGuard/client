import './style.scss';
import clsx from 'clsx';
import { useMemo } from 'react';
import { useAppData } from '../../../../providers/AppDataContext';
import type { LocationInfo } from '../../../../rust-api/types';
import { decideLocationMfaMethod } from '../../../../utils/decideLocationMfaMethod';
import { mfaToText } from '../../../../utils/mfa';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  onEdit: () => void;
}

export const LocationCardMfaEdit = ({ location, onEdit, variant }: Props) => {
  const { locationMfaPreference } = useAppData();

  const mfaMethod = useMemo(
    () => decideLocationMfaMethod(location, locationMfaPreference[String(location.id)]),
    [locationMfaPreference, location],
  );

  if (location.location_mfa_mode === 'disabled' || mfaMethod === null) return null;

  return (
    <div className={clsx('location-card-mfa-edit', `variant-${variant}`)}>
      <div className="mfa-badge">
        <p>MFA</p>
      </div>
      <p className="name">{mfaToText(mfaMethod)}</p>
      {location.location_mfa_mode === 'internal' && !location.active && (
        <IconButton
          variant={IconButtonVariant.SmallSelected}
          icon="edit"
          onClick={onEdit}
        />
      )}
    </div>
  );
};
