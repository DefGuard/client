import './style.scss';
import clsx from 'clsx';
import type { LocationInfo } from '../../../../rust-api/types';
import { mfaStepCount, mfaStepsToText, mfaToText } from '../../../../utils/mfa';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  onEdit: () => void;
}

export const LocationCardMfaEdit = ({ location, onEdit, variant }: Props) => {
  const stepCount = mfaStepCount(location);
  const isMultiStep = stepCount > 1;

  const label = isMultiStep
    ? mfaStepsToText(stepCount)
    : location.mfa_method && mfaToText(location.mfa_method);

  const canEdit = isMultiStep
    ? location.mfa_steps.some((step) => step.methods.length > 1)
    : location.location_mfa_mode === 'internal';

  if ((location.location_mfa_mode === 'disabled' && !isMultiStep) || !label) return null;

  return (
    <div className={clsx('location-card-mfa-edit', `variant-${variant}`)}>
      <div className="mfa-badge">
        <p>MFA</p>
      </div>
      <p className="name">{label}</p>
      {canEdit && !location.active && (
        <IconButton
          variant={IconButtonVariant.SmallSelected}
          icon="edit"
          onClick={onEdit}
        />
      )}
    </div>
  );
};
