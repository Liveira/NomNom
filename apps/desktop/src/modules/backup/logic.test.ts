import { describe, expect, it } from 'vitest';
import { ago, inTime, leaf, newPlan } from './logic';

describe('backup helpers', () => {
  it('says how long ago something happened', () => {
    expect(ago(1000, 1030)).toBe('just now');
    expect(ago(1000, 1000 + 600)).toBe('10 minutes ago');
    expect(ago(1000, 1000 + 7200)).toBe('2 hours ago');
    expect(ago(1000, 1000 + 100_000)).toBe('yesterday');
    expect(ago(1000, 1000 + 5 * 86_400)).toBe('5 days ago');
  });

  it('says how long until the next run', () => {
    expect(inTime(1000, 990)).toBe('now');
    expect(inTime(1000 + 1800, 1000)).toBe('in 30 minutes');
    expect(inTime(1000 + 7200, 1000)).toBe('in 2 hours');
    expect(inTime(1000 + 3 * 86_400, 1000)).toBe('in 3 days');
  });

  it('shows a folder by its name', () => {
    expect(leaf('C:\\Users\\me\\Pictures')).toBe('Pictures');
    expect(leaf('/home/me/Documents/')).toBe('Documents');
    expect(leaf('')).toBe('');
  });

  it('starts a new plan with sensible choices', () => {
    const p = newPlan();
    expect(p.schedule).toBe('daily');
    expect(p.keep).toBe(10);
    expect(p.sources).toEqual([]);
  });
});
