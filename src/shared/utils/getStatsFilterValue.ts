import dayjs from 'dayjs';
import utc from 'dayjs/plugin/utc';

dayjs.extend(utc);

export const getStatsFilterValue = (hours: number): string =>
  dayjs.utc().subtract(hours, 'hours').toISOString();
