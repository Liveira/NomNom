import { describe, expect, it } from 'vitest';
import { IDLE_CHOICES, fileNameOf } from './logic';

describe('phone transfer helpers', () => {
  it('offers short to long waits with no repeats, and 15 minutes among them', () => {
    const m = IDLE_CHOICES.map((c) => c.minutes);
    expect(m).toEqual([...m].sort((a, b) => a - b));
    expect(new Set(m).size).toBe(m.length);
    expect(m).toContain(15);
  });
  it('takes a file name from either kind of path', () => {
    expect(fileNameOf('C:\\a\\b.png')).toBe('b.png');
    expect(fileNameOf('/a/b.png')).toBe('b.png');
  });
});
