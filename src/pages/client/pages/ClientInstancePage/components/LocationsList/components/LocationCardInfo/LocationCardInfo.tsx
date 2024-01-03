import './style.scss';

import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';

import { useI18nContext } from '../../../../../../../../i18n/i18n-react';
import { clientApi } from '../../../../../../clientAPI/clientApi';
import { clientQueryKeys } from '../../../../../../query';
import {
  CommonWireguardFields,
  Connection,
  WireguardInstanceType,
} from '../../../../../../types';

type Props = {
  location?: CommonWireguardFields;
  connection?: Connection;
};

const { getActiveConnection } = clientApi;

export const LocationCardInfo = ({ location, connection }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.connectionLabels;

  const { data: activeConnection } = useQuery({
    queryKey: [
      clientQueryKeys.getActiveConnection,
      location?.id as number,
      location?.connection_type,
    ],
    queryFn: () =>
      getActiveConnection({
        locationId: location?.id as number,
        connectionType: location?.connection_type as WireguardInstanceType,
      }),
    enabled: location?.active,
  });

  return (
    <>
      <div className="location-card-info-from">
        <label>
          {location?.active ? localLL.connectedFrom() : localLL.lastConnectedFrom()}:
        </label>
        {location?.active ? (
          <p>{activeConnection?.connected_from}</p>
        ) : (
          <p>{connection ? connection.connected_from : localLL.neverConnected()}</p>
        )}
      </div>
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
        <p>{location?.address}</p>
      </div>
    </>
  );
};
