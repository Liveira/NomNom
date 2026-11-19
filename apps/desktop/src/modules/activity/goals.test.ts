import { describe, expect, it } from 'vitest';
import { describeGoal, fillOf, minutesText, sinceLastWeek } from './goals';

describe('goal text', () => {
  it('writes times the way people say them', () => {
    expect(minutesText(45)).toBe('45 min');
    expect(minutesText(120)).toBe('2 h');
    expect(minutesText(135)).toBe('2 h 15 min');
  });
  it('describes a limit and a target', () => {
    expect(describeGoal({ id: 'a', scope: 'all', kind: 'limit', period: 'day', minutes: 120, enabled: true })).toBe('At most 2 h a day');
    expect(describeGoal({ id: 'a', scope: 'all', kind: 'target', period: 'week', minutes: 600, enabled: true })).toBe('At least 10 h a week');
  });
  it('keeps the bar between empty and full', () => {
    const p = (percent: number) => ({ used_secs: 0, goal_secs: 1, percent, state: 'under' as const });
    expect(fillOf(p(-5))).toBe(0);
    expect(fillOf(p(42))).toBe(42);
    expect(fillOf(p(480))).toBe(100);
  });
});

describe('the weekly comparison', () => {
  const report = (total: number, before: number) => ({ week_start: '2026-11-09', total_secs: total, previous_total_secs: before, days: [0, 0, 0, 0, 0, 0, 0], top_apps: [], goals: [] });
  it('shows a rise, a fall, and nothing without a week before', () => {
    expect(sinceLastWeek(report(120, 100))).toBe('+20% on the week before');
    expect(sinceLastWeek(report(50, 100))).toBe('-50% on the week before');
    expect(sinceLastWeek(report(50, 0))).toBeNull();
  });
});
