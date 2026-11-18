import { describe, expect, it } from 'vitest';
import type { StTree } from '../../lib/ipc';
import { pieSlices, sunburstArcs } from './charts';

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

describe('sunburstArcs', () => {
  const node = (id: number, name: string, size: number, children: StTree[] = []): StTree => ({
    id,
    name,
    is_dir: children.length > 0,
    size,
    alloc: size,
    category: children.length > 0 ? 'folder' : 'video',
    children,
    hidden_count: 0,
    hidden_size: 0,
    hidden_alloc: 0,
  });

  it('builds concentric arcs down to maxDepth with proportional shares', () => {
    const root = node(0, 'C:', 100, [node(1, 'Movies', 60, [node(3, 'a.mp4', 40), node(4, 'b.mp4', 20)]), node(2, 'Docs', 40)]);
    const arcs = sunburstArcs(root, 'size', 3);
    expect(arcs.filter((a) => a.depth === 1).map((a) => a.name)).toEqual(['Movies', 'Docs']);
    expect(arcs.filter((a) => a.depth === 2).map((a) => a.name)).toEqual(['a.mp4', 'b.mp4']);
    expect(arcs.find((a) => a.name === 'Movies')!.share).toBeCloseTo(0.6, 5);
  });
});
