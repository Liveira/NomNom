import { describe, expect, it } from 'vitest';
import { pieSlices } from './charts';

describe('pieSlices', () => {
  it('shares add up to one and follow the values', () => {
    const s = pieSlices([{ key: 'a', value: 50 }, { key: 'b', value: 30 }, { key: 'c', value: 20 }]);
    expect(s.map((x) => x.share)).toEqual([0.5, 0.3, 0.2]);
    expect(s.reduce((n, x) => n + x.share, 0)).toBeCloseTo(1, 10);
  });
  it('leaves out empty items and gives nothing for an empty list', () => {
    expect(pieSlices([{ key: 'a', value: 0 }])).toEqual([]);
    expect(pieSlices([{ key: 'a', value: 0 }, { key: 'b', value: 5 }]).map((x) => x.key)).toEqual(['b']);
  });
  it('uses the large-arc flag for a slice over half the circle', () => {
    const [big, small] = pieSlices([{ key: 'a', value: 75 }, { key: 'b', value: 25 }]);
    expect(big.path).toContain('A 1 1 0 1 1');
    expect(small.path).toContain('A 1 1 0 0 1');
  });
  it('draws a single item as a full circle', () => {
    const [only] = pieSlices([{ key: 'a', value: 10 }]);
    expect(only.share).toBe(1);
    expect(only.path.match(/A 1 1/g)).toHaveLength(2);
  });
});
