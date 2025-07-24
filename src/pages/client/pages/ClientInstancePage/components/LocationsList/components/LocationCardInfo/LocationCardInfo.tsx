import './style.scss';

import dayjs from 'dayjs';
import { useMemo } from 'react';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { FloatingMenu } from '../../../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenu';
import { FloatingMenuProvider } from '../../../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuProvider';
import { FloatingMenuTrigger } from '../../../../../../../../shared/defguard-ui/components/Layout/FloatingMenu/FloatingMenuTrigger';
import type { CommonWireguardFields, Connection } from '../../../../../../types';

type Props = {
  location?: CommonWireguardFields;
  connection?: Connection;
};

export const LocationCardInfo = ({ location, connection }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.connectionLabels;

  return (
    <>
      <div className="location-card-info-connected">
        <label>{localLL.lastConnected()}:</label>
        <p>
          {location?.active
            ? localLL.active()
            : connection
              ? dayjs.utc(connection.end).local().format('DD-MM-YYYY')
              : localLL.neverConnected()}
        </p>
      </div>
      <div className="location-card-info-ip">
        <label>{localLL.assignedIp()}:</label>
        <AssignedIp address={location?.address || ''} />
      </div>
    </>
  );
};

type AssignedIpProps = {
  // comma-separated list of addresses
  address: string;
};

const AssignedIp = ({ address }: AssignedIpProps) => {
  // split into separate addreses to show in tooltip
  const addresses = useMemo(() => address.split(','), [address]);

  return (
    <FloatingMenuProvider placement="right" disabled={addresses.length === 0}>
      <FloatingMenuTrigger asChild>
        <div className="assigned-ip-container">
          <p className="client-addresses">{address}</p>
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
