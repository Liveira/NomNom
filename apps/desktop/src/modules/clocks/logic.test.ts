import { describe, expect, it } from 'vitest';
import { clockText, isZone, momentIn, offsetLabel, offsetMinutes, parseDuration, stopwatchText } from './logic';

describe('time zones', () => {
  it('knows real zones and refuses made-up ones', () => {
    expect(isZone('America/Sao_Paulo')).toBe(true);
    expect(isZone('Mars/Olympus')).toBe(false);
  });
  it('finds the offset, summer time included', () => {
    expect(offsetMinutes('UTC', new Date('2026-06-01T12:00:00Z'))).toBe(0);
    expect(offsetMinutes('Asia/Tokyo', new Date('2026-06-01T12:00:00Z'))).toBe(540);
    expect(offsetMinutes('America/New_York', new Date('2026-01-15T12:00:00Z'))).toBe(-300);
    expect(offsetMinutes('America/New_York', new Date('2026-07-15T12:00:00Z'))).toBe(-240);
    expect(offsetMinutes('Asia/Kolkata', new Date('2026-06-01T12:00:00Z'))).toBe(330);
  });
  it('writes an offset for people', () => {
    expect(offsetLabel(0)).toBe('UTC+0');
    expect(offsetLabel(-180)).toBe('UTC-3');
    expect(offsetLabel(330)).toBe('UTC+5:30');
  });
  it('turns a wall-clock time in a zone into the moment', () => {
    expect(momentIn('2026-01-15T09:00', 'America/New_York')?.toISOString()).toBe('2026-01-15T14:00:00.000Z');
    expect(momentIn('2026-07-15T09:00', 'America/New_York')?.toISOString()).toBe('2026-07-15T13:00:00.000Z');
    expect(momentIn('2026-06-01T21:00', 'Asia/Tokyo')?.toISOString()).toBe('2026-06-01T12:00:00.000Z');
    expect(momentIn('nonsense', 'UTC')).toBeNull();
  });
});

describe('durations', () => {
  it('reads the usual ways to write one', () => {
    expect(parseDuration('90')).toBe(90 * 60);
    expect(parseDuration('25m')).toBe(1500);
    expect(parseDuration('1h30')).toBe(5400);
    expect(parseDuration('1h 30m')).toBe(5400);
    expect(parseDuration('45s')).toBe(45);
    expect(parseDuration('1:30:00')).toBe(5400);
    expect(parseDuration('10:00')).toBe(600);
    expect(parseDuration('2h')).toBe(7200);
  });
  it('turns away nonsense', () => {
    expect(parseDuration('')).toBeNull();
    expect(parseDuration('soon')).toBeNull();
    expect(parseDuration('1h30x')).toBeNull();
  });
});

describe('clock text', () => {
  it('writes a countdown', () => {
    expect(clockText(65)).toBe('01:05');
    expect(clockText(3909)).toBe('1:05:09');
    expect(clockText(-4)).toBe('00:00');
  });
  it('writes the stopwatch with hundredths', () => {
    expect(stopwatchText(65_420)).toBe('01:05.42');
    expect(stopwatchText(3_665_000)).toBe('1:01:05.00');
    expect(stopwatchText(0)).toBe('00:00.00');
  });
});
