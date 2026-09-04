import './style.scss';
import clsx from 'clsx';
import type { LocationInfo } from '../../../../rust-api/types';
import {
  mfaStepCount,
  mfaStepsToText,
  mfaToText,
  resolveMfaStepPlan,
  usableMfaMethods,
} from '../../../../utils/mfa';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  onEdit: () => void;
}

export const LocationCardMfaEdit = ({ location, onEdit, variant }: Props) => {
  const stepCount = mfaStepCount(location);
  const label =
    stepCount > 1
      ? mfaStepsToText(stepCount)
      : mfaToText(resolveMfaStepPlan(location)[0]);

  const canEdit = location.mfa_steps.some((step) => usableMfaMethods(step).length > 1);

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
