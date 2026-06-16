import './style.scss';
import clsx from 'clsx';
import { useEffect, useMemo } from 'react';
import { type LocationInfo, MfaMethod } from '../../../../rust-api/types';
import { useSharedStorage } from '../../../../store/useSharedStorage';
import { mfaToText } from '../../../../utils/mfa';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  onEdit: () => void;
}

export const LocationCardMfaEdit = ({ location, onEdit, variant }: Props) => {
  const methodSelection = useSharedStorage((s) => s.locationMethodSelection);
  const mfaMethod = useMemo(
    () => methodSelection[location.id] ?? MfaMethod.Totp,
    [methodSelection, location.id],
  );

  useEffect(() => {
    console.log(mfaMethod, methodSelection);
  }, [mfaMethod, methodSelection]);

  if (location.location_mfa_mode === 'disabled') return null;

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
