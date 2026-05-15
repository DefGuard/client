import { useQuery } from '@tanstack/react-query';
import { BarElement, CategoryScale, Chart as ChartJS, LinearScale } from 'chart.js';
import { Fragment } from 'react/jsx-runtime';
import { Bar } from 'react-chartjs-2';
import { getLocationStatsQueryOptions } from '../../../../rust-api/query';
import type { ConnectionType } from '../../../../rust-api/types';
import { ThemeSpacing } from '../../../../types';
import { SizedBox } from '../../../SizedBox/SizedBox';

ChartJS.register(BarElement, CategoryScale, LinearScale);

const BAR_COLOR = 'rgba(255, 255, 255, 0.20)';

interface Props {
  locationId: number;
  connectionType: ConnectionType;
}

export const ConnectionChart = ({ locationId, connectionType }: Props) => {
  const { data: stats } = useQuery(
    getLocationStatsQueryOptions({ locationId, connectionType }),
  );

  const chartData = {
    labels: stats?.map((s) => s.collected_at) ?? [],
    datasets: [
      {
        label: 'upload',
        data: stats?.map((s) => s.upload) ?? [],
        backgroundColor: BAR_COLOR,
        borderWidth: 0,
        borderRadius: 0,
        categoryPercentage: 0.95,
        barPercentage: 1.0,
      },
      {
        label: 'download',
        data: stats?.map((s) => s.download) ?? [],
        backgroundColor: BAR_COLOR,
        borderWidth: 0,
        borderRadius: 0,
        categoryPercentage: 0.95,
        barPercentage: 1.0,
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
    <Fragment>
      <div style={{ width: '100%', height: '30px' }}>
        <Bar data={chartData} options={options} />
      </div>
      <SizedBox height={ThemeSpacing.Lg} />
    </Fragment>
  );
};
