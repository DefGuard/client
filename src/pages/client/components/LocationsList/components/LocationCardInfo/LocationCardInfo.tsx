import './style.scss';

import { useI18nContext } from '../../../../../../i18n/i18n-react';
import { DefguardLocation } from '../../../../types';

type Props = {
  location: DefguardLocation;
};

export const LocationCardInfo = ({ location }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.connectionLabels;
  return (
    <>
      <div className="location-card-info-from">
        <label>{localLL.lastConnectedFrom()}:</label>
        <p>placeholder host ip</p>
      </div>
      <div className="location-card-info-connected">
        <label>{localLL.lastConnected()}:</label>
        <p>placeholder</p>
      </div>
      <div className="location-card-info-ip">
        <label>{localLL.assignedIp()}:</label>
        <p>Placeholder</p>
      </div>
    </>
  );
};
