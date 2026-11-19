import { describe, expect, it } from 'vitest';
import { KEEP_CHOICES, ago } from './logic';

describe('ago', () => {
  const now = 1_800_000_000;
  it('says how long ago in the words people use', () => {
    expect(ago(now - 10, now)).toBe('just now');
    expect(ago(now - 5 * 60, now)).toBe('5 min ago');
    expect(ago(now - 3 * 3600, now)).toBe('3 h ago');
    expect(ago(now - 26 * 3600, now)).toBe('yesterday');
    expect(ago(now - 5 * 86_400, now)).toBe('5 days ago');
  });
  it('never goes negative', () => {
    expect(ago(now + 500, now)).toBe('just now');
  });
});

describe('how long to keep', () => {
  it('offers a day up to for ever, with for ever last', () => {
    expect(KEEP_CHOICES[0].days).toBe(1);
    expect(KEEP_CHOICES.at(-1)?.days).toBe(0);
    expect(new Set(KEEP_CHOICES.map((c) => c.days)).size).toBe(KEEP_CHOICES.length);
  });
});
