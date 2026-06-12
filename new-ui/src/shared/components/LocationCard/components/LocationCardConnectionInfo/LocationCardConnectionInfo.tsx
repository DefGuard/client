import './style.scss';
import { useQuery } from '@tanstack/react-query';
import dayjs from 'dayjs';
import { useId, useMemo } from 'react';
import { api } from '../../../../rust-api/api';
import { getLocationStatsQueryOptions } from '../../../../rust-api/query';
import type { LocationInfo } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { SizedBox } from '../../../SizedBox/SizedBox';
import { ConnectionChart } from '../ConnectionChart/ConnectionChart';

export const LocationCardConnectionInfo = ({ location }: { location: LocationInfo }) => {
  const { data: stats } = useQuery(
    getLocationStatsQueryOptions({
      locationId: location.id,
      connectionType: location.connection_type,
    }),
  );

  const { data: lastConnection } = useQuery({
    queryKey: ['locations', location.id, 'last-connect'],
    queryFn: () =>
      api.getLastConnection({
        connectionType: location.connection_type,
        locationId: location.id,
      }),
  });

  const lastConnectedText = useMemo(() => {
    if (!lastConnection) return 'Never';
    return dayjs.utc(lastConnection.end).local().format('DD MMM YYYY');
  }, [lastConnection]);

  if (!stats || stats.length === 0)
    return (
      <div className="no-connection-info">
        <EmptyIcon />
        <SizedBox height={ThemeSpacing.Xl} />
        <p className="title">{`Traffic data not available`}</p>
        <SizedBox height={ThemeSpacing.Xs} />
        <p className="description">{`Connect once to see the traffic details.`}</p>
      </div>
    );

  return (
    <div className="location-card-connection-info">
      <div className="connection-info">
        <div className="info">
          <div className="label">Last connected</div>
          <div className="label-value">{lastConnectedText}</div>
        </div>
        <div className="info">
          <div className="label">Assigned IP</div>
          <div className="label-value">{location.address}</div>
        </div>
      </div>
      <SizedBox height={ThemeSpacing.Xl} />
      <ConnectionChart stats={stats} />
    </div>
  );
};

const EmptyIcon = () => {
  const id = useId();
  return (
    <svg
      width="48"
      height="48"
      viewBox="0 0 48 48"
      fill="none"
      xmlns="http://www.w3.org/2000/svg"
    >
      <rect
        x="0.5"
        y="0.5"
        width="47"
        height="47"
        rx="23.5"
        stroke="white"
        strokeOpacity="0.4"
        strokeDasharray="2 2"
      />
      <g clipPath={`url(#${id})`}>
        <path
          d="M30.0866 17.1307H20.9842C19.099 17.1307 17.5708 18.7012 17.5708 20.6384V29.9923C17.5708 31.9295 19.099 33.5 20.9842 33.5H30.0866C31.9718 33.5 33.5 31.9295 33.5 29.9923V20.6384C33.5 18.7012 31.9718 17.1307 30.0866 17.1307Z"
          fill="white"
          fillOpacity="0.1"
        />
        <path
          d="M28.8692 15.377H19.7668C17.8816 15.377 16.3534 16.9474 16.3534 18.8846V28.2385C16.3534 30.1757 17.8816 31.7462 19.7668 31.7462H28.8692C30.7544 31.7462 32.2826 30.1757 32.2826 28.2385V18.8846C32.2826 16.9474 30.7544 15.377 28.8692 15.377Z"
          stroke="white"
          strokeLinejoin="round"
        />
        <path
          d="M31.873 23.5499H27.7314L25.5354 28.4957L23.1005 18.6274L20.9046 23.5733H16.3534"
          stroke="white"
          strokeLinejoin="round"
        />
      </g>
      <defs>
        <clipPath id={id}>
          <rect width="18" height="19" fill="white" transform="translate(15.5 14.5)" />
        </clipPath>
      </defs>
    </svg>
  );
};
