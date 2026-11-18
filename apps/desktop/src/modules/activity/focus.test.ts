import { describe, expect, it } from 'vitest';
import { clock, formatHours, ringOffset } from './focus';

describe('focus timer helpers', () => {
  it('writes a countdown as minutes and seconds, or with hours', () => {
    expect(clock(25 * 60)).toBe('25:00');
    expect(clock(61)).toBe('01:01');
    expect(clock(0)).toBe('00:00');
    expect(clock(3725)).toBe('1:02:05');
    expect(clock(-5)).toBe('00:00');
  });

  it('empties the ring as time runs out', () => {
    expect(ringOffset(100, 1)).toBe(0);
    expect(ringOffset(100, 0.25)).toBe(75);
    expect(ringOffset(100, 0)).toBe(100);
    expect(ringOffset(100, 4)).toBe(0);
    expect(ringOffset(100, -1)).toBe(100);
  });

  it('shortens a stretch of time', () => {
    expect(formatHours(0)).toBe('0m');
    expect(formatHours(45)).toBe('45s');
    expect(formatHours(25 * 60)).toBe('25m');
    expect(formatHours(3600)).toBe('1h');
    expect(formatHours(3900)).toBe('1h 5m');
  });
});
