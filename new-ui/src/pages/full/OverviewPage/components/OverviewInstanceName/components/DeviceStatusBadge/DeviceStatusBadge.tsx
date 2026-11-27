import { Badge } from '../../../../../../../shared/components/Badge/Badge';
import { BadgeVariant } from '../../../../../../../shared/components/Badge/types';
import {
  DeviceStatus,
  type DeviceStatusValue,
} from '../../../../../../../shared/rust-api/types';

type Props = {
  status: DeviceStatusValue | null;
};

export const DeviceStatusBadge = ({ status }: Props) => {
  switch (status) {
    case DeviceStatus.Verified:
      return <Badge text="Verified" variant={BadgeVariant.Success} />;
    case DeviceStatus.Blocked:
      return <Badge text="Blocked" variant={BadgeVariant.Critical} />;
    default:
      return null;
  }
};
