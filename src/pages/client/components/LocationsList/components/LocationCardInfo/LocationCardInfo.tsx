import './style.scss';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { DefguardLocation, Connection } from '../../../../types';
import dayjs from 'dayjs';

type Props = {
  location: DefguardLocation;
  connection?: Connection;
};

export const LocationCardInfo = ({ location, connection }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.connectionLabels;
  return (
    <>
      <div className="location-card-info-from">
        <label>
          {location.active ? localLL.connectedFrom() : localLL.lastConnectedFrom()}:
        </label>
        <p>{connection ? connection.connected_from : 'Never connected'}</p>
      </div>
      <div className="location-card-info-connected">
        <label>{localLL.lastConnected()}:</label>
        <p>
          {location.active
            ? 'Active'
            : connection
            ? dayjs.utc(connection.end).local().format('DD-MM-YYYY')
            : 'Never connected'}
        </p>
      </div>
      <div className="location-card-info-ip">
        <label>{localLL.assignedIp()}:</label>
        <p>{location.address}</p>
      </div>
    </>
  );
};
