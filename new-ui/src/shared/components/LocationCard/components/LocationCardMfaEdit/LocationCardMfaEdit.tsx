import './style.scss';
import clsx from 'clsx';
import { TooltipContent } from '../../../../providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../providers/tooltip/TooltipTrigger';
import type { InstanceInfo, LocationInfo } from '../../../../rust-api/types';
import {
  ConnectionAbility,
  type ConnectionAbilityValue,
  mfaStepCount,
  mfaStepsOf,
  mfaStepsToText,
  mfaToText,
  resolveMfaStepPlan,
  usableMfaMethods,
} from '../../../../utils/mfa';
import { IconKind } from '../../../Icon';
import { IconButton } from '../../../IconButton/IconButton';
import { IconButtonVariant } from '../../../IconButton/types';

interface Props {
  variant: 'compact' | 'full';
  location: LocationInfo;
  instance?: Pick<InstanceInfo, 'mfa_configured_methods'>;
  /** Supplied by the caller - the tray card reads it off the LocationCard context,
   *  the desktop card computes it with `useConnectionAbility`. */
  connectionAbility: ConnectionAbilityValue;
  onEdit: () => void;
}

const CONFIGURE_REQUIRED_TOOLTIP =
  'Access requires a new MFA method set by your administrator. Please set it up to continue.';

export const LocationCardMfaEdit = ({
  location,
  instance,
  onEdit,
  variant,
  connectionAbility,
}: Props) => {
  const stepCount = mfaStepCount(location);
  const label =
    stepCount > 1
      ? mfaStepsToText(stepCount)
      : mfaToText(resolveMfaStepPlan(location)[0]);

  const canEdit =
    mfaStepsOf(location).some((step) => usableMfaMethods(step, instance).length > 1) &&
    connectionAbility === ConnectionAbility.Available;

  const canConfigure = connectionAbility === ConnectionAbility.Configurable;

  return (
    <div className={clsx('location-card-mfa-edit', `variant-${variant}`)}>
      <div className="mfa-badge">
        <p>MFA</p>
      </div>
      <p className="name">{label}</p>
      {canConfigure && (
        <TooltipProvider placement="top">
          <TooltipTrigger>
            <IconButton variant={IconButtonVariant.Small} icon={IconKind.InfoOutlined} />
          </TooltipTrigger>
          <TooltipContent>
            <p>{CONFIGURE_REQUIRED_TOOLTIP}</p>
          </TooltipContent>
        </TooltipProvider>
      )}
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
