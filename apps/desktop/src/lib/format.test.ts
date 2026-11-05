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

import { progressLine } from './format';

describe('progressLine', () => {
  it('joins the known parts with a spaced middle dot', () => {
    expect(progressLine({ fraction: 0.62, speedBps: 2.4e6, etaSecs: 4, etaReliable: true })).toBe('62% · 2.3 MB/s · 4 s left');
    expect(progressLine({ fraction: 0.62, speedBps: 2.4e6, etaSecs: 4, etaReliable: false })).toBe('62% · 2.3 MB/s');
  });
  it('says Starting when nothing is known yet', () => {
    expect(progressLine({ fraction: null, speedBps: null, etaSecs: null, etaReliable: false })).toBe('Starting');
  });
  it('never leaves a dangling separator', () => {
    expect(progressLine({ fraction: 0.5, speedBps: null, etaSecs: null, etaReliable: false })).toBe('50%');
  });
});
