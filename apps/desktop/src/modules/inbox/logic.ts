import type { NoticeItem } from '../../lib/ipc';

export interface Day {
  /** The calendar day, as 2026-11-19. */
  key: string;
  daysAgo: number;
  items: NoticeItem[];
}

const dayKey = (d: Date) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`;

/** The notices (newest first) split by the day they came, newest day first. */
export function groupByDay(items: NoticeItem[], nowSec: number): Day[] {
  const today = new Date(nowSec * 1000);
  today.setHours(0, 0, 0, 0);
  const days: Day[] = [];
  for (const n of items) {
    const d = new Date(n.at * 1000);
    const key = dayKey(d);
    let last = days[days.length - 1];
    if (!last || last.key !== key) {
      const start = new Date(d);
      start.setHours(0, 0, 0, 0);
      last = { key, daysAgo: Math.round((today.getTime() - start.getTime()) / 86_400_000), items: [] };
      days.push(last);
    }
    last.items.push(n);
  }
  return days;
}

export function dayLabel(key: string, daysAgo: number): string {
  if (daysAgo <= 0) return 'Today';
  if (daysAgo === 1) return 'Yesterday';
  const [y, m, d] = key.split('-').map(Number);
  return new Date(y, m - 1, d).toLocaleDateString(undefined, { weekday: 'long', day: 'numeric', month: 'long' });
}
