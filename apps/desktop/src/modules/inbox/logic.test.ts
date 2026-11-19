import { describe, expect, it } from 'vitest';
import type { NoticeItem } from '../../lib/ipc';
import { dayLabel, groupByDay } from './logic';

const at = (y: number, mo: number, d: number, h: number) => Math.floor(new Date(y, mo - 1, d, h, 0, 0).getTime() / 1000);
const item = (id: number, when: number): NoticeItem => ({ id, at: when, kind: 'info', title: 't', body: '', action: null });
const now = at(2026, 11, 19, 15);

describe('groupByDay', () => {
  it('splits by calendar day, newest first, and counts the days back', () => {
    const list = [item(5, at(2026, 11, 19, 14)), item(4, at(2026, 11, 19, 9)), item(3, at(2026, 11, 18, 23)), item(2, at(2026, 11, 12, 8))];
    const days = groupByDay(list, now);
    expect(days.map((d) => [d.daysAgo, d.items.map((i) => i.id)])).toEqual([
      [0, [5, 4]],
      [1, [3]],
      [7, [2]],
    ]);
  });
  it('is empty for nothing', () => {
    expect(groupByDay([], now)).toEqual([]);
  });
});

describe('dayLabel', () => {
  it('names today and yesterday and writes the date for older days', () => {
    expect(dayLabel('2026-11-19', 0)).toBe('Today');
    expect(dayLabel('2026-11-18', 1)).toBe('Yesterday');
    expect(dayLabel('2026-11-12', 7)).toMatch(/12/);
  });
});
