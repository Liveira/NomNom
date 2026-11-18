import { describe, expect, it } from 'vitest';
import { parseLocal, suggestLocal, whenText } from './when';

describe('scheduled download times', () => {
  const noon = new Date(2026, 10, 18, 12, 0, 0);

  it('says today, tomorrow or the date', () => {
    const at = (d: number, h: number) => Math.floor(new Date(2026, 10, 18 + d, h, 30).getTime() / 1000);
    expect(whenText(at(0, 21), noon)).toMatch(/^at .* today$/);
    expect(whenText(at(1, 8), noon)).toMatch(/^at .* tomorrow$/);
    expect(whenText(at(5, 8), noon)).toMatch(/^on .* at /);
  });

  it('reads the value of a time field', () => {
    expect(parseLocal('2026-11-19T21:30')).toBe(Math.floor(new Date(2026, 10, 19, 21, 30).getTime() / 1000));
    expect(parseLocal('')).toBeNull();
    expect(parseLocal('tomorrow')).toBeNull();
  });

  it('suggests a time an hour ahead, on a round five minutes', () => {
    expect(suggestLocal(new Date(2026, 10, 18, 12, 2, 10))).toBe('2026-11-18T13:05');
    expect(suggestLocal(new Date(2026, 10, 18, 23, 58, 0))).toBe('2026-11-19T01:00');
  });
});
