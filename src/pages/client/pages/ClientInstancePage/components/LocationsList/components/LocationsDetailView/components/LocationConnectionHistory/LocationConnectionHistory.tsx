import { useQuery } from '@tanstack/react-query';
import './style.scss';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { clientApi } from '../../../../../../../../clientAPI/clientApi';
import { clientQueryKeys } from '../../../../../../../../query';
import { DefguardLocation } from '../../../../../../../../types';
import { LocationCardNeverConnected } from '../../../LocationCardNeverConnected/LocationCardNeverConnected';
import { LocationHistoryTable } from './LocationHistoryTable/LocationHistoryTable';

type Props = {
  locationId: DefguardLocation['id'];
  connected: boolean;
};

const { getConnectionHistory } = clientApi;

export const LocationConnectionHistory = ({ locationId, connected }: Props) => {
  const { LL } = useI18nContext();
  const localLL = LL.pages.client.pages.instancePage.detailView.history;

  const { data: connectionHistory } = useQuery({
    queryKey: [clientQueryKeys.getConnectionHistory, locationId],
    queryFn: () => getConnectionHistory({ locationId }),
    enabled: !!locationId,
    refetchInterval: 10 * 1000,
    refetchOnWindowFocus: true,
    refetchOnMount: true,
  });

  if (!connectionHistory) return null;

  return (
    <Card id="connection-history-card">
      <header>
        <h2>{localLL.title()}</h2>
      </header>
      {connectionHistory.length === 0 && !connected && <LocationCardNeverConnected />}
      {connectionHistory.length > 0 && (
        <LocationHistoryTable connections={connectionHistory} />
      )}
    </Card>
  );
};
