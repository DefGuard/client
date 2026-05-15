import './style.scss';
import { useQuery } from '@tanstack/react-query';
import { BarElement, CategoryScale, Chart as ChartJS, LinearScale } from 'chart.js';
import { sum } from 'radashi';
import { useMemo } from 'react';
import { Bar } from 'react-chartjs-2';
import { getLocationStatsQueryOptions } from '../../../../rust-api/query';
import type { ConnectionType } from '../../../../rust-api/types';
import { BoxIcon } from '../../../BoxIcon/BoxIcon';
import { Icon, IconKind } from '../../../Icon';
import { TransferText } from '../../../TransferText/TransferText';

ChartJS.register(BarElement, CategoryScale, LinearScale);

const UPLOAD_COLOR = 'rgba(255, 255, 255, 0.20)';
const DOWNLOAD_COLOR = 'rgba(255, 255, 255, 1.0)';

interface Props {
  locationId: number;
  connectionType: ConnectionType;
}

export const ConnectionChart = ({ locationId, connectionType }: Props) => {
  const { data: stats } = useQuery(
    getLocationStatsQueryOptions({ locationId, connectionType }),
  );

  const statsSum = useMemo(
    () => ({
      download: sum(stats ?? [], (s) => s.download),
      upload: sum(stats ?? [], (s) => s.upload),
    }),
    [stats],
  );

  const chartData = {
    labels: stats?.map((s) => s.collected_at) ?? [],
    datasets: [
      {
        label: 'upload',
        data: stats?.map((s) => s.upload) ?? [],
        backgroundColor: UPLOAD_COLOR,
        borderWidth: 0,
        borderRadius: 0,
        categoryPercentage: 0.95,
        barPercentage: 1.0,
        maxBarThickness: 2.2,
      },
      {
        label: 'download',
        data: stats?.map((s) => s.download) ?? [],
        backgroundColor: DOWNLOAD_COLOR,
        borderWidth: 0,
        borderRadius: 0,
        categoryPercentage: 0.95,
        barPercentage: 1.0,
        maxBarThickness: 2.2,
      },
    ],
  };

  const options = {
    responsive: true,
    maintainAspectRatio: false,
    animation: false as const,
    layout: { padding: 0 },
    plugins: {
      legend: { display: false },
      tooltip: { enabled: false },
    },
    scales: {
      x: {
        display: false,
        grid: { display: false },
        border: { display: false },
      },
      y: {
        display: false,
        grid: { display: false },
        border: { display: false },
      },
    },
  };

  if (!stats?.length) return null;

  return (
    <div className="connection-chart">
      <div className="chart-container" style={{ width: '100%', height: '30px' }}>
        <Bar data={chartData} options={options} />
      </div>
      <div className="stats-summary">
        <div className="summary">
          <BoxIcon>
            <Icon icon={IconKind.ArrowBig} rotationDirection="down" />
          </BoxIcon>
          <TransferText data={statsSum.download} />
        </div>
        <div className="summary">
          <BoxIcon>
            <Icon icon={IconKind.ArrowBig} rotationDirection="up" />
          </BoxIcon>
          <TransferText data={statsSum.upload} />
        </div>
      </div>
    </div>
  );
};
