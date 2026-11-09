import './style.scss';
import { Badge } from '../../../../../../../shared/components/Badge/Badge';
import { Divider } from '../../../../../../../shared/components/Divider/Divider';
import { IconKind } from '../../../../../../../shared/components/Icon';
import { Toggle } from '../../../../../../../shared/components/Toggle/Toggle';
import { TooltipContent } from '../../../../../../../shared/providers/tooltip/TooltipContent';
import { TooltipProvider } from '../../../../../../../shared/providers/tooltip/TooltipContext';
import { TooltipTrigger } from '../../../../../../../shared/providers/tooltip/TooltipTrigger';
import {
  AutoConnectBlock,
  type LocationInfo,
} from '../../../../../../../shared/rust-api/types';
import { ThemeSpacing } from '../../../../../../../shared/types';
import { isPresent } from '../../../../../../../shared/utils/isPresent';

type Props = {
  name: string;
  locations: LocationInfo[];
  disabled: boolean;
  onToggle: (location: LocationInfo) => void;
};

export const AutoConnectInstanceCard = ({
  name,
  locations,
  disabled,
  onToggle,
}: Props) => {
  return (
    <div className="auto-connect-instance-card">
      <p className="title">{name}</p>
      <Divider spacing={ThemeSpacing.Md} />
      <div className="locations">
        {locations.map((location) => (
          <LocationRow
            key={location.id}
            location={location}
            disabled={disabled}
            onToggle={() => onToggle(location)}
          />
        ))}
      </div>
    </div>
  );
};

type LocationRowProps = {
  location: LocationInfo;
  disabled: boolean;
  onToggle: () => void;
};

const LocationRow = ({ location, disabled, onToggle }: LocationRowProps) => {
  const blocked = location.auto_connect_blocked;

  return (
    <div className="location">
      <Toggle
        label={location.name}
        active={location.auto_connect}
        disabled={disabled || isPresent(blocked)}
        onClick={onToggle}
      />
      {blocked === AutoConnectBlock.Mfa && (
        <LockedBadge
          text="MFA required"
          tooltip="Only locations without MFA can auto-connect."
        />
      )}
      {blocked === AutoConnectBlock.AllTrafficTaken && (
        <LockedBadge
          text="All traffic auto-connect taken"
          tooltip="Another All traffic location, possibly in a different instance, already auto-connects or is managed by the Defguard service."
        />
      )}
      {!isPresent(blocked) && (
        <Badge
          text={`${location.routes_all_traffic ? 'All traffic' : 'Predefined traffic'} / No MFA`}
        />
      )}
    </div>
  );
};

type LockedBadgeProps = {
  text: string;
  tooltip: string;
};

const LockedBadge = ({ text, tooltip }: LockedBadgeProps) => {
  return (
    <TooltipProvider placement="top">
      <TooltipTrigger>
        <Badge disabled icon={IconKind.LockClosed} text={text} />
      </TooltipTrigger>
      <TooltipContent>
        <p>{tooltip}</p>
      </TooltipContent>
    </TooltipProvider>
  );
};
