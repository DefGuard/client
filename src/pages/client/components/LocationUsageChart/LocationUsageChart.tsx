import dayjs from 'dayjs';
import { Bar, BarChart, XAxis, YAxis } from 'recharts';

import { ColorsRGB } from '../../../../shared/constants';
import { LocationStats } from '../../types';
import { sortBy } from 'lodash-es';
import { useMemo } from 'react';

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
  console.log(filtered);
  const formatted = filtered.map((stats) => ({
    ...stats,
    collected_at: dayjs.utc(stats.collected_at).toDate().getTime(),
  }));
  return sortBy(formatted, ['collected_at']);
};

export const LocationUsageChart = ({
  data,
  height = 400,
  width = 900,
  hideX = false,
  barSize = 20,
  heightX = 50,
}: LocationUsageProps) => {
  const getFormattedData = useMemo(() => parseStatsForCharts(data), [data]);

  return (
    <div className="location-usage">
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
          axisLine={{ stroke: ColorsRGB.GrayBorder }}
          tickLine={{ stroke: ColorsRGB.GrayBorder }}
          hide={hideX}
          padding={{ left: 0, right: 0 }}
          tick={{ fontSize: 10, color: ColorsRGB.GrayLight }}
          tickFormatter={formatXTick}
          domain={['dataMin', 'dataMax']}
          interval={'preserveStartEnd'}
        />
        <YAxis
          hide={true}
          domain={['dataMin', 'dataMax']}
          padding={{ top: 0, bottom: 0 }}
        />
        <Bar dataKey="download" fill={ColorsRGB.Primary} />
        <Bar dataKey="upload" fill={ColorsRGB.Error} />
      </BarChart>
    </div>
  );
};

const formatXTick = (tickData: number) => dayjs.utc(tickData).local().format('HH:mm');
