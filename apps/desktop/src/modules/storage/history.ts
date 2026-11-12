/** What the History tab says about snapshots and their differences. Pure, so it can be tested. */
import { formatBytes } from '../../lib/format';
import type { StChange, StComparison, StSnapshot } from '../../lib/ipc';

/** "+1.2 MB", "-300 KB" or "no change". */
export function signedBytes(n: number): string {
  if (n === 0) return 'no change';
  return `${n > 0 ? '+' : '-'}${formatBytes(Math.abs(n))}`;
}

export const CHANGE_LABELS: Record<StChange, string> = { grew: 'Bigger', shrank: 'Smaller', new: 'New', gone: 'Gone' };

export function dayText(unix: number): string {
  return new Date(unix * 1000).toLocaleDateString([], { year: 'numeric', month: 'short', day: 'numeric' });
}
export function dateTimeText(unix: number): string {
  return new Date(unix * 1000).toLocaleString([], { year: 'numeric', month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

export function snapshotName(m: StSnapshot): string {
  const when = dateTimeText(m.created_at);
  return m.label ? `${m.label}, ${when}` : when;
}

/** One sentence on the whole comparison. `afterName` is "the scan open now" when there is no second snapshot. */
export function summaryLine(c: StComparison, beforeName: string, afterName: string): string {
  const what = c.delta === 0 ? 'The total size did not change' : `The total ${c.delta > 0 ? 'grew' : 'shrank'} by ${formatBytes(Math.abs(c.delta))}`;
  return `${what}, from ${formatBytes(c.bytes_before)} (${beforeName}) to ${formatBytes(c.bytes_after)} (${afterName}).`;
}

/** What was and wasn't compared, so nobody reads more into the list than it says. */
export function scopeNote(c: StComparison): string {
  return `Folders down to ${c.depth} ${c.depth === 1 ? 'level' : 'levels'} and files of ${formatBytes(c.big_min)} or more were compared. Smaller files and deeper folders were not kept in both snapshots, so they are not listed. The total above covers everything.`;
}

export interface Bar {
  /** 0 to 1 of the tallest bar. */
  height: number;
  /** The snapshot didn't keep this folder. */
  missing: boolean;
}

export function trendBars(points: { size: number | null }[]): Bar[] {
  const max = Math.max(0, ...points.map((p) => p.size ?? 0));
  return points.map((p) => ({ height: p.size == null || max === 0 ? 0 : p.size / max, missing: p.size == null }));
}

export function scheduleLabel(s: string): string {
  switch (s) {
    case 'on_start': return 'When NomNom starts';
    case 'daily': return 'Once a day';
    case 'weekly': return 'Once a week';
    default: return 'Off';
  }
}

/** The error count of a snapshot, in words, or '' when there were none. */
export function errorsNote(m: Pick<StSnapshot, 'errors'>): string {
  return m.errors > 0 ? `${m.errors.toLocaleString()} ${m.errors === 1 ? 'place' : 'places'} couldn't be read` : '';
}
