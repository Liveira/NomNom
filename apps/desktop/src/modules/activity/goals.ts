import type { GoalView, PlayGoal, WeekReport } from '../../lib/ipc';

const DAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
export const dayName = (i: number): string => DAYS[i] ?? '';

/** "1 h 20 min" for a number of minutes. */
export function minutesText(total: number): string {
  const h = Math.floor(total / 60);
  const m = total % 60;
  if (h === 0) return `${m} min`;
  return m === 0 ? `${h} h` : `${h} h ${m} min`;
}

/** "At most 2 h a day". */
export function describeGoal(g: PlayGoal): string {
  const when = g.period === 'day' ? 'a day' : 'a week';
  return `${g.kind === 'limit' ? 'At most' : 'At least'} ${minutesText(g.minutes)} ${when}`;
}

/** How full the bar is, 0 to 100. */
export function fillOf(p: GoalView['progress']): number {
  return Math.max(0, Math.min(100, p.percent));
}

/** How much more (or less) than the week before, in percent, when there was a week before. */
export function changePercent(r: WeekReport): number | null {
  if (r.previous_total_secs <= 0) return null;
  return Math.round(((r.total_secs - r.previous_total_secs) * 100) / r.previous_total_secs);
}

/** "+20% on the week before", or nothing when there is no week before. */
export function sinceLastWeek(r: WeekReport): string | null {
  const pct = changePercent(r);
  return pct === null ? null : `${pct >= 0 ? '+' : ''}${pct}% on the week before`;
}
