import './style.scss';

import classNames from 'classnames';
import { useMemo } from 'react';

import { Badge } from '../../../../../../../../shared/defguard-ui/components/Layout/Badge/Badge';
import { BadgeStyleVariant } from '../../../../../../../../shared/defguard-ui/components/Layout/Badge/types';
import { FloatingMenu } from '../../../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import SvgIconConnection from '../../../../../../../../shared/defguard-ui/components/svg/IconConnection';
import type { CommonWireguardFields } from '../../../../../../types';

type Props = {
  location?: CommonWireguardFields;
};

export const LocationCardTitle = ({ location }: Props) => {
  const cn = classNames('location-card-title', {
    active: location?.active,
  });
  return (
    <div className={cn}>
      <SvgIconConnection />
      <span className="location-name">{location?.name}</span>

      <AddressBadge addresses={location?.address || ''} />
    </div>
  );
};

type AddressBadgeProps = {
  // comma-separated list of addresses
  addresses: string;
};

const AddressBadge = ({ addresses: address }: AddressBadgeProps) => {
  // split into separate addreses to show in tooltip
  const addresses = useMemo(() => address.split(','), [address]);

  return (
    <FloatingMenuProvider placement="right" disabled={addresses.length === 0}>
      <FloatingMenuTrigger asChild>
        <div className="addresses-badge-container">
          <Badge
            className="client-addresses"
            text={address}
            styleVariant={BadgeStyleVariant.STANDARD}
            // ref={containerRef}
          />
        </div>
      </FloatingMenuTrigger>
      <FloatingMenu>
        <ul className="list-addresses-floating">
          {addresses.map((d) => (
            <li key={d}>{d}</li>
          ))}
        </ul>
      </FloatingMenu>
    </FloatingMenuProvider>
  );
};
