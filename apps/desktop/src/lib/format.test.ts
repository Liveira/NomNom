import { describe, expect, it } from 'vitest';
import { formatBytes, formatDuration, formatEta, formatSpeed } from './format';

describe('format helpers', () => {
  it('formats sizes', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(1536)).toBe('1.5 KB');
    expect(formatBytes(5 * 1024 * 1024)).toBe('5.0 MB');
    expect(formatBytes(250 * 1024 * 1024)).toBe('250 MB');
  });
  it('returns nothing for unknown values instead of inventing them', () => {
    expect(formatBytes(null)).toBe('');
    expect(formatBytes(undefined)).toBe('');
    expect(formatDuration(null)).toBe('');
    expect(formatSpeed(null)).toBe('');
    expect(formatEta(null)).toBe('');
  });
  it('formats durations', () => {
    expect(formatDuration(65)).toBe('1:05');
    expect(formatDuration(3725)).toBe('1:02:05');
  });
  it('formats eta', () => {
    expect(formatEta(30)).toBe('30 s left');
    expect(formatEta(600)).toBe('10 min left');
    expect(formatEta(7500)).toBe('2 h 5 min left');
  });
});
