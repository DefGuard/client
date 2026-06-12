import type { Duration } from 'dayjs/plugin/duration';

export function formatDuration(dur: Duration): string {
  if (dur.days() > 0) return dur.format('D[d] H[h]');
  if (dur.hours() > 0) return dur.format('H[h] m[min]');
  if (dur.minutes() > 0) return dur.format('m[min] s[sec]');
  return dur.format('s[sec]');
}
