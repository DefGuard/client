import './style.scss';

import { useQuery } from '@tanstack/react-query';
import classNames from 'classnames';
import parse from 'html-react-parser';
import { memo } from 'react';
import { Label } from 'recharts';
import { useBreakpoint } from 'use-breakpoint';

import { useI18nContext } from '../../../../../../../../../../i18n/i18n-react';
import { deviceBreakpoints } from '../../../../../../../../../../shared/constants';
import { Card } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Card/Card';
import { Helper } from '../../../../../../../../../../shared/defguard-ui/components/Layout/Helper/Helper';
import { getStatsFilterValue } from '../../../../../../../../../../shared/utils/getStatsFilterValue';
import { clientApi } from '../../../../../../../../clientAPI/clientApi';
import { useClientStore } from '../../../../../../../../hooks/useClientStore';
import { clientQueryKeys } from '../../../../../../../../query';
import { CommonWireguardFields, DefguardInstance } from '../../../../../../../../types';
import { LocationUsageChart } from '../../../../../LocationUsageChart/LocationUsageChart';
import { LocationUsageChartType } from '../../../../../LocationUsageChart/types';
import { LocationCardConnectButton } from '../../../LocationCardConnectButton/LocationCardConnectButton';
import { LocationCardInfo } from '../../../LocationCardInfo/LocationCardInfo';
import { LocationCardNoStats } from '../../../LocationCardNoStats/LocationCardNoStats';
import { LocationCardRoute } from '../../../LocationCardRoute/LocationCardRoute';
import { LocationCardTitle } from '../../../LocationCardTitle/LocationCardTitle';

type Props = {
  location: CommonWireguardFields;
  tabbed?: boolean;
  selectedDefguardInstance?: DefguardInstance;
};

const { getLocationStats, getLastConnection } = clientApi;

export const LocationDetailCard = memo(
  ({ location, tabbed = false, selectedDefguardInstance }: Props) => {
    const { LL } = useI18nContext();
    const { breakpoint } = useBreakpoint({ ...deviceBreakpoints, desktop: 1300 });
    const localLL = LL.pages.client.pages.instancePage;
    const statsFilter = useClientStore((state) => state.statsFilter);

    const { data: locationStats } = useQuery({
      queryKey: [
        clientQueryKeys.getLocationStats,
        location.id,
        statsFilter,
        location.connection_type,
      ],
      queryFn: () =>
        getLocationStats({
          locationId: location.id,
          connectionType: location.connection_type,
          from: getStatsFilterValue(statsFilter),
        }),
      enabled: !!location,
      refetchInterval: 10 * 1000,
      refetchOnWindowFocus: true,
      refetchOnMount: true,
    });

    const { data: lastConnection } = useQuery({
      queryKey: [clientQueryKeys.getConnections, location.id, location.connection_type],
      queryFn: () =>
        getLastConnection({
          locationId: location.id,
          connectionType: location.connection_type,
        }),
      enabled: !!location,
      refetchInterval: 10 * 1000,
      refetchOnWindowFocus: true,
      refetchOnMount: true,
    });

    return (
      <Card
        className={classNames('detail-card', {
          tabbed,
        })}
      >
        <div className="header">
          <LocationCardTitle location={location} />
          {breakpoint === 'desktop' && (
            <LocationCardInfo location={location} connection={lastConnection} />
          )}
          {breakpoint === 'desktop' && (
            <div className="route">
              {!location?.active && (
                <div className="controls">
                  <Helper initialPlacement="left">
                    {parse(localLL.controls.traffic.helper())}
                  </Helper>
                  <LocationCardRoute location={location} />
                </div>
              )}
              {location?.active && (
                <div className="location-card-allowed-traffic">
                  <label>{localLL.controls.traffic.label()}:</label>
                  <p>
                    {location.route_all_traffic
                      ? localLL.controls.traffic.allTraffic()
                      : localLL.controls.traffic.predefinedTraffic()}
                  </p>
                </div>
              )}
            </div>
          )}
          <LocationCardConnectButton location={location} />
        </div>
        {breakpoint !== 'desktop' && (
          <div className="info">
            <LocationCardInfo location={location} connection={lastConnection} />
          </div>
        )}
        {breakpoint !== 'desktop' && (
          <div className="route">
            <div className="top">
              <Label>{localLL.controls.traffic.label()}</Label>
              <Helper
                initialPlacement="right"
                icon={
                  <svg
                    xmlns="http://www.w3.org/2000/svg"
                    width={12}
                    height={12}
                    fill="none"
                  >
                    <path
                      style={{
                        fill: 'var(--surface-icon-primary)',
                      }}
                      d="M6 12A6 6 0 1 0 6 0a6 6 0 0 0 0 12Z"
                    />
                    <path
                      style={{
                        fill: 'var(--surface-icon-secondary)',
                      }}
                      d="M6.667 5.333a.667.667 0 0 0-1.334 0v3.334a.667.667 0 0 0 1.334 0V5.333ZM6.667 3.333a.667.667 0 1 0-1.334 0 .667.667 0 0 0 1.334 0Z"
                    />
                  </svg>
                }
              >
                {parse(LL.pages.client.pages.instancePage.controls.traffic.helper())}
              </Helper>
            </div>
            <LocationCardRoute
              location={location}
              selectedDefguardInstance={selectedDefguardInstance}
            />
          </div>
        )}
        {locationStats && locationStats.length > 0 ? (
          <LocationUsageChart
            data={locationStats}
            type={LocationUsageChartType.LINE}
            margin={{ left: 20, right: 20 }}
          />
        ) : (
          <div className="no-stats-container">
            <LocationCardNoStats />
          </div>
        )}
      </Card>
    );
  },
);
