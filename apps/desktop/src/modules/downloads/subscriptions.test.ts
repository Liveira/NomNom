import { describe, expect, it } from 'vitest';
import { EVERY_CHOICES, checkedText } from './subscriptions';

describe('subscriptions text', () => {
  const now = 1_800_000_000;
  it('says when it last looked', () => {
    expect(checkedText(null, 6, now)).toBe('Not looked at yet');
    expect(checkedText(now - 30, 6, now)).toBe('Looked just now');
    expect(checkedText(now - 20 * 60, 6, now)).toBe('Looked 20 min ago');
    expect(checkedText(now - 5 * 3600, 6, now)).toBe('Looked 5 h ago');
  });
  it('offers hourly up to weekly, shortest first, with no repeats', () => {
    const hours = EVERY_CHOICES.map((c) => c.hours);
    expect(hours).toEqual([...hours].sort((a, b) => a - b));
    expect(new Set(hours).size).toBe(hours.length);
    expect(hours[0]).toBe(1);
  });
});
