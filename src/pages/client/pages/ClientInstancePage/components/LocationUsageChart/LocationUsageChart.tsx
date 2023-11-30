import './style.scss';

import dayjs from 'dayjs';
import { sortBy } from 'lodash-es';
import { useMemo } from 'react';
import AutoSizer from 'react-virtualized-auto-sizer';
import { Bar, BarChart, Line, LineChart, XAxis, YAxis } from 'recharts';

import { NetworkSpeed } from '../../../../../../shared/defguard-ui/components/Layout/NetworkSpeed/NetworkSpeed';
import { NetworkDirection } from '../../../../../../shared/defguard-ui/components/Layout/NetworkSpeed/types';
import { useTheme } from '../../../../../../shared/defguard-ui/hooks/theme/useTheme';
import { LocationStats } from '../../../../types';
import { LocationUsageChartType } from './types';

type ChartBoxSpacing = {
  top?: number;
  bottom?: number;
  left?: number;
  right?: number;
};

interface LocationUsageProps {
  data: LocationStats[];
  type: LocationUsageChartType;
  hideX?: boolean;
  barSize?: number;
  barGap?: number;
  heightX?: number;
  margin?: ChartBoxSpacing;
  padding?: ChartBoxSpacing;
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
  hideX = false,
  barSize = 2,
  barGap = 2,
  heightX = 50,
  type,
  margin,
  padding,
}: LocationUsageProps) => {
  const [totalUpload, totalDownload] = useMemo(() => totalUploadDownload(data), [data]);
  const getFormattedData = useMemo(() => parseStatsForCharts(data), [data]);
  const { colors } = useTheme();

  const getMargin = useMemo((): ChartBoxSpacing => {
    const defaultMargin: ChartBoxSpacing = {
      top: 0,
      left: 0,
      right: 0,
      bottom: 0,
    };
    return margin ?? defaultMargin;
  }, [margin]);

  const getPadding = useMemo((): ChartBoxSpacing => {
    const defaultPadding: ChartBoxSpacing = {
      bottom: 0,
      right: 0,
      left: 0,
      top: 0,
    };
    return padding ?? defaultPadding;
  }, [padding]);

  if (!data.length) return null;
  return (
    <div className="location-usage">
      <div className="summary">
        <NetworkSpeed speedValue={totalDownload} direction={NetworkDirection.DOWNLOAD} />
        <NetworkSpeed speedValue={totalUpload} direction={NetworkDirection.UPLOAD} />
      </div>
      {type === LocationUsageChartType.BAR && (
        <AutoSizer>
          {(size) => (
            <BarChart
              width={size.width}
              height={size.height}
              data={getFormattedData}
              margin={getMargin}
              barSize={barSize}
              barGap={barGap}
            >
              <XAxis
                dataKey="collected_at"
                scale="time"
                type="number"
                height={heightX}
                width={size.width}
                axisLine={{ stroke: colors.surfaceDefaultModal }}
                tickLine={{ stroke: colors.surfaceDefaultModal }}
                hide={hideX}
                padding={getPadding}
                tick={{
                  fontSize: 12,
                  color: '#222',
                  fontWeight: 500,
                  fontFamily: 'Roboto',
                }}
                tickFormatter={formatXTick}
                domain={['dataMin', 'dataMax']}
                interval={'preserveEnd'}
              />
              <YAxis
                hide={true}
                domain={['dataMin', 'dataMax']}
                padding={{ top: 0, bottom: 0 }}
              />
              <Bar dataKey="download" fill={colors.surfaceMainPrimary} />
              <Bar dataKey="upload" fill={colors.textAlert} />
            </BarChart>
          )}
        </AutoSizer>
      )}

      {type === LocationUsageChartType.LINE && (
        <AutoSizer>
          {(size) => (
            <LineChart
              width={size.width}
              height={size.height}
              data={getFormattedData}
              margin={getMargin}
            >
              <XAxis
                dataKey="collected_at"
                scale="time"
                type="number"
                height={heightX}
                width={size.width}
                axisLine={{ stroke: colors.surfaceDefaultModal }}
                tickLine={{ stroke: colors.surfaceDefaultModal }}
                hide={hideX}
                padding={getPadding}
                dx={3}
                tick={{
                  fontSize: 12,
                  color: '#222',
                  fontWeight: 500,
                  fontFamily: 'Roboto',
                }}
                tickFormatter={formatXTick}
                domain={['dataMin', 'dataMax']}
                interval={'equidistantPreserveStart'}
              />
              <YAxis
                hide={true}
                domain={['dataMin', 'dataMax']}
                padding={{ top: 0, bottom: 0 }}
              />
              <Line
                dataKey="download"
                stroke={colors.surfaceMainPrimary}
                strokeWidth={1}
                dot={false}
              />
              <Line
                dataKey="upload"
                stroke={colors.textAlert}
                strokeWidth={1}
                dot={false}
              />
            </LineChart>
          )}
        </AutoSizer>
      )}
    </div>
  );
};

// FIXME: hack with spaces to avoid tick overlapping
const formatXTick = (tickData: number) =>
  dayjs.utc(tickData).local().format('HH:mm:ss  ');
