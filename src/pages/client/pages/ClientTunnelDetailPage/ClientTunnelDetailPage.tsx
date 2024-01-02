import { useQuery } from '@tanstack/react-query';
import { useParams } from 'react-router-dom';

import { useI18nContext } from '../../../../i18n/i18n-react';
import { clientApi } from '../../clientAPI/clientApi';
import { clientQueryKeys } from '../../query';
import { WireguardInstanceType } from '../../types';
import { LocationConnectionHistory } from '../ClientInstancePage/components/LocationsList/components/LocationsDetailView/components/LocationConnectionHistory/LocationConnectionHistory';
import { LocationDetailCard } from '../ClientInstancePage/components/LocationsList/components/LocationsDetailView/components/LocationDetailCard/LocationDetailCard';
import { LocationDetails } from '../ClientInstancePage/components/LocationsList/components/LocationsDetailView/components/LocationDetails/LocationDetails';
import { StatsFilterSelect } from '../ClientInstancePage/components/StatsFilterSelect/StatsFilterSelect';

const { getTunnels } = clientApi;

export const ClientTunnelDetailPage = () => {
  const { id } = useParams();
  const { LL } = useI18nContext();
  const pageLL = LL.pages.client.pages.tunnelPage;
  const { data: tunnels } = useQuery({
    queryKey: [clientQueryKeys.getTunnels],
    queryFn: () => {
      return getTunnels();
    },
  });

  const tunnel = tunnels?.find((tunnel) => tunnel.id === Number(id));

  return (
    <section id="client-instance-page" className="client-page">
      <header>
        <h1>{pageLL.title()}</h1>
        <div className="options">
          <StatsFilterSelect />
        </div>
      </header>
      <div id="locations-detail-view">
        {tunnel && (
          <>
            <LocationDetailCard location={tunnel} />
            <LocationConnectionHistory
              locationId={tunnel?.id}
              connected={tunnel?.active}
              locationType={tunnel.location_type}
            />
            <LocationDetails
              locationId={tunnel?.id}
              locationType={WireguardInstanceType.TUNNEL}
            />
          </>
        )}
      </div>
    </section>
  );
};
