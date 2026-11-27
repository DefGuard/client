import './style.scss';
import { TruncatedText } from '../../../../../shared/components/TruncatedText/TruncatedText';
import type { DeviceStatusValue } from '../../../../../shared/rust-api/types';
import { DeviceStatusBadge } from './components/DeviceStatusBadge/DeviceStatusBadge';

type Props = {
  name: string;
  deviceStatus: DeviceStatusValue | null;
};

export const OverviewInstanceName = ({ name, deviceStatus }: Props) => {
  return (
    <div className="overview-instance-name">
      <TruncatedText
        className="name"
        text={name}
        tooltipPlacement="bottom-start"
        delayTooltip
      />
      <DeviceStatusBadge status={deviceStatus} />
    </div>
  );
};
