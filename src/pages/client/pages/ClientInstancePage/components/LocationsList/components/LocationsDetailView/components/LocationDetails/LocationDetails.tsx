import './style.scss';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Divider } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { DefguardLocation } from '../../../../../../../../types';
import { LocationLogs } from '../LocationLogs/LocationLogs';
import { Label } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Label/Label';

type Props = {
  locationId: DefguardLocation['id'];
};

export const LocationDetails = ({ locationId }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details;

  return (
    <Card id="location-details-card">
      <header>
        <h2>{localLL.title()}</h2>
      </header>
      <LocationLogs locationId={locationId} />
      <div className="info-section">
        <h3>{localLL.info.configuration.title()}</h3>
        <div className="info">
          <Label>{localLL.info.configuration.pubkey()}</Label>
          <div className="values pubkey">
            <p></p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.configuration.address()}</Label>
          <div className="values">
            <p></p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.configuration.listenPort()}</Label>
          <div className="values">
            <p></p>
          </div>
        </div>
      </div>
      <Divider />
      <div className="info-section">
        <h3>{localLL.info.vpn.title()}</h3>
        <div className="info">
          <Label>{localLL.info.vpn.pubkey()}</Label>
          <div className="values pubkey">
            <p></p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.allowedIps()}</Label>
          <div className="values ips">
            <p></p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.dns()}</Label>
          <div className="values">
            <p></p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.keepalive()}</Label>
          <div className="values">
            <p></p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.handshake()}</Label>
          <div className="values">
            <p></p>
          </div>
        </div>
      </div>
    </Card>
  );
};
