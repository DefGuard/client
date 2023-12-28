import './style.scss';

import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { memo } from 'react';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Divider } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Divider/Divider';
import { Label } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Label/Label';
import { clientApi } from '../../../../../../../../clientAPI/clientApi';
import { clientQueryKeys } from '../../../../../../../../query';
import { DefguardLocation } from '../../../../../../../../types';
import { LocationLogs } from '../LocationLogs/LocationLogs';

type Props = {
  locationId: DefguardLocation['id'];
};

const { getLocationDetails } = clientApi;

export const LocationDetails = ({ locationId }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details;

  return (
    <Card id="location-details-card">
      <header>
        <h2>{localLL.title()}</h2>
      </header>
      <LocationLogs locationId={locationId} />
      <InfoSection locationId={locationId} />
    </Card>
  );
};

const InfoSection = memo(({ locationId }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.details;

  const { data } = useQuery({
    queryKey: [clientQueryKeys.getLocationDetails, locationId],
    queryFn: () => getLocationDetails({ locationId }),
    enabled: !!locationId,
    refetchInterval: 1000,
  });

  const handshakeString = () => {
    if (data) {
      const handshake = data.last_handshake && dayjs.unix(data.last_handshake);
      const now = dayjs();
      return localLL.info.vpn.handshakeValue({ seconds: now.diff(handshake, 'seconds') });
    }
    return '';
  };

  return (
    <>
      <div className="info-section">
        <h3>{localLL.info.configuration.title()}</h3>
        <div className="info">
          <Label>{localLL.info.configuration.pubkey()}</Label>
          <div className="values pubkey">
            <p>{data?.peer_pubkey}</p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.configuration.address()}</Label>
          <div className="values">
            <p>{data?.peer_endpoint}</p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.configuration.listenPort()}</Label>
          <div className="values">
            <p>{data?.listen_port}</p>
          </div>
        </div>
      </div>
      <Divider />
      <div className="info-section">
        <h3>{localLL.info.vpn.title()}</h3>
        <div className="info">
          <Label>{localLL.info.vpn.pubkey()}</Label>
          <div className="values pubkey">
            <p>{data?.pubkey}</p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.allowedIps()}</Label>
          <div className="values ips">
            {data && data.address.split(',').map((ip) => <p key={ip}>{ip}</p>)}
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.dns()}</Label>
          <div className="values">
            <p>{data && data.dns && data.dns.map((d) => <p key={d}>{d}</p>)}</p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.keepalive()}</Label>
          <div className="values">
            <p>{data?.persistent_keepalive_interval ?? ''}</p>
          </div>
        </div>
        <div className="info">
          <Label>{localLL.info.vpn.handshake()}</Label>
          <div className="values">
            <p>{handshakeString()}</p>
          </div>
        </div>
      </div>
    </>
  );
});
