import dayjs from 'dayjs';
import { sortBy } from 'lodash-es';
import { useMemo } from 'react';
import { Bar, BarChart, ResponsiveContainer, XAxis, YAxis } from 'recharts';

import { NetworkSpeed } from '../../../../shared/defguard-ui/components/Layout/NetworkSpeed/NetworkSpeed';
import { NetworkDirection } from '../../../../shared/defguard-ui/components/Layout/NetworkSpeed/types';
import { useTheme } from '../../../../shared/defguard-ui/hooks/theme/useTheme';
import { LocationStats } from '../../types';

interface LocationUsageProps {
  data: LocationStats[];
  width?: number;
  height?: number;
  hideX?: boolean;
  barSize?: number;
  heightX?: number;
}

const parseStatsForCharts = (data: LocationStats[]): LocationStats[] => {
  const filtered = data.filter((stats) => stats.download > 0 || stats.upload > 0);
  const formatted = filtered.map((stats) => ({
    ...stats,
    collected_at: dayjs.utc(stats.collected_at).toDate().getTime(),
  }));
  return sortBy(formatted, ['collected_at']);
};

const totalUploadDownload = (data: LocationStats[]): number[] => {
  let totalDownload = 0;
  let totalUpload = 0;
  for (const locationStat of data) {
    totalDownload += locationStat.download;
    totalUpload += locationStat.upload;
  }
  return [totalUpload, totalDownload];
};

export const LocationUsageChart = ({
  data,
  height = 300,
  width = 900,
  hideX = false,
  barSize = 10,
  heightX = 50,
}: LocationUsageProps) => {
  const [totalUpload, totalDownload] = useMemo(() => totalUploadDownload(data), [data]);
  const getFormattedData = useMemo(() => parseStatsForCharts(data), [data]);
  const { colors } = useTheme();
  return (
    <div className="location-usage">
      <div className="summary">
        <>
          <NetworkSpeed
            speedValue={totalDownload}
            direction={NetworkDirection.DOWNLOAD}
          />
          <NetworkSpeed speedValue={totalUpload} direction={NetworkDirection.UPLOAD} />
        </>
      </div>
      <ResponsiveContainer width="100%" height={height}>
        <BarChart
          height={height}
          width={width}
          data={getFormattedData}
          margin={{ bottom: 0, left: 0, right: 0, top: 0 }}
          barGap={2}
          barSize={barSize}
        >
          <XAxis
            dataKey="collected_at"
            scale="time"
            type="number"
            height={heightX}
            width={width}
            axisLine={{ stroke: colors.surfaceDefaultModal }}
            tickLine={{ stroke: colors.surfaceDefaultModal }}
            hide={hideX}
            padding={{ left: 0, right: 0 }}
            tick={{ fontSize: 10, color: '#000000' }}
            tickFormatter={formatXTick}
            domain={['dataMin', 'dataMax']}
            interval={'preserveEnd'}
          />
          <YAxis
            hide={true}
            domain={['dataMin', 'dataMax']}
            padding={{ top: 0, bottom: 0 }}
          />
          <Bar barSize={barSize} dataKey="download" fill={colors.surfaceMainPrimary} />
          <Bar barSize={barSize} dataKey="upload" fill={colors.textAlert} />
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
};

const formatXTick = (tickData: number) => dayjs.utc(tickData).local().format('HH:mm');
