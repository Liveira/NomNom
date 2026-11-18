import { describe, expect, it } from 'vitest';
import {
  categoryInfo,
  formatDecimalHours,
  formatHourSlot,
  formatHours,
  heatmapLevel,
  monogram,
  relativeAgo,
} from './logic';

describe('playtime formatting and buckets', () => {
  it('formats active seconds into clear human durations', () => {
    expect(formatHours(0)).toBe('0m');
    expect(formatHours(42)).toBe('42s');
    expect(formatHours(1800)).toBe('30m');
    expect(formatHours(7200)).toBe('2h');
    expect(formatHours(5490)).toBe('1h 31m');
    expect(formatDecimalHours(9000)).toBe('2.5h');
  });

  it('computes relative timestamps and hour slots', () => {
    expect(relativeAgo(1000, 1030)).toBe('Just now');
    expect(relativeAgo(1000, 2200)).toBe('20m ago');
    expect(relativeAgo(1000, 11800)).toBe('3h ago');
    expect(formatHourSlot(21)).toBe('21:00 – 22:00');
    expect(formatHourSlot(null)).toBe('—');
  });

  it('assigns heatmap levels and monograms', () => {
    expect(heatmapLevel(0)).toBe(0);
    expect(heatmapLevel(900)).toBe(1);
    expect(heatmapLevel(3600)).toBe(2);
    expect(heatmapLevel(10800)).toBe(3);
    expect(heatmapLevel(25000)).toBe(4);
    expect(monogram('Clip Studio Paint')).toBe('CS');
    expect(monogram('Discord')).toBe('DI');
    expect(categoryInfo('creative').label).toBe('Creative');
  });
});
