import { describe, expect, it } from 'vitest';
import type { StTree } from '../../lib/ipc';
import { hitTest, layoutTree, resolveColor, squarify, type Rect } from './treemap';

const area = (r: Rect) => r.w * r.h;
const overlap = (a: Rect, b: Rect) => {
  const w = Math.min(a.x + a.w, b.x + b.w) - Math.max(a.x, b.x);
  const h = Math.min(a.y + a.h, b.y + b.h) - Math.max(a.y, b.y);
  return w > 1e-6 && h > 1e-6;
};

describe('squarify', () => {
  const box: Rect = { x: 10, y: 20, w: 400, h: 300 };
  const values = [60, 30, 20, 10, 5, 5, 1];

  it('gives every item an area proportional to its value and fills the box', () => {
    const placed = squarify(values, (v) => v, box);
    const total = values.reduce((a, b) => a + b, 0);
    expect(placed).toHaveLength(values.length);
    for (const p of placed) expect(area(p.rect) / area(box)).toBeCloseTo(p.item / total, 6);
    expect(placed.reduce((s, p) => s + area(p.rect), 0)).toBeCloseTo(area(box), 4);
  });

  it('keeps everything inside the box and nothing overlaps', () => {
    const placed = squarify(values, (v) => v, box);
    for (const p of placed) {
      expect(p.rect.x).toBeGreaterThanOrEqual(box.x - 1e-6);
      expect(p.rect.y).toBeGreaterThanOrEqual(box.y - 1e-6);
      expect(p.rect.x + p.rect.w).toBeLessThanOrEqual(box.x + box.w + 1e-6);
      expect(p.rect.y + p.rect.h).toBeLessThanOrEqual(box.y + box.h + 1e-6);
    }
    for (let i = 0; i < placed.length; i++) for (let j = i + 1; j < placed.length; j++) expect(overlap(placed[i].rect, placed[j].rect)).toBe(false);
  });

  it('puts the biggest first and skips empty items and empty boxes', () => {
    const placed = squarify([1, 0, 9, 5], (v) => v, box);
    expect(placed.map((p) => p.item)).toEqual([9, 5, 1]);
    expect(squarify([1, 2], (v) => v, { x: 0, y: 0, w: 0, h: 10 })).toEqual([]);
    expect(squarify([], (v: number) => v, box)).toEqual([]);
  });

  it('handles one item and thousands of items', () => {
    const one = squarify([7], (v) => v, box)[0].rect;
    for (const k of ['x', 'y', 'w', 'h'] as const) expect(one[k]).toBeCloseTo(box[k], 6);
    const many = Array.from({ length: 5000 }, (_, i) => 1 + (i % 17));
    const placed = squarify(many, (v) => v, box);
    expect(placed).toHaveLength(5000);
    expect(placed.reduce((s, p) => s + area(p.rect), 0)).toBeCloseTo(area(box), 2);
  });
});

const tree = (id: number, name: string, size: number, children: StTree[] = [], extra: Partial<StTree> = {}): StTree => ({
  id,
  name,
  is_dir: children.length > 0,
  size,
  alloc: size,
  category: children.length ? 'folder' : 'video',
  children,
  hidden_count: 0,
  hidden_size: 0,
  hidden_alloc: 0,
  ...extra,
});

describe('layoutTree and colour modes', () => {
  const root = tree(0, 'root', 100, [tree(1, 'a', 60, [tree(3, 'x', 40), tree(4, 'y', 20)]), tree(2, 'b', 40)]);
  const box: Rect = { x: 0, y: 0, w: 800, h: 600 };

  it('nests children inside their folder', () => {
    const cells = layoutTree(root, box, 'size');
    const byId = new Map(cells.map((c) => [c.id, c]));
    const a = byId.get(1)!.rect;
    const x = byId.get(3)!.rect;
    expect(x.x).toBeGreaterThanOrEqual(a.x);
    expect(x.y).toBeGreaterThan(a.y);
    expect(x.x + x.w).toBeLessThanOrEqual(a.x + a.w + 1e-6);
    expect(byId.get(3)!.depth).toBe(2);
    expect(byId.get(3)!.parent).toBe(1);
  });

  it('shows folded small items and optional free/system drive blocks', () => {
    const t = tree(0, 'root', 100, [tree(1, 'big', 90)], { hidden_count: 7, hidden_size: 10, hidden_alloc: 10, is_dir: true });
    const cells = layoutTree(t, box, 'size', 3, { freeBytes: 50, systemBytes: 25 });
    const folded = cells.find((c) => c.folded)!;
    expect(folded.foldedCount).toBe(7);
    expect(folded.name).toBe('7 smaller items');
    expect(cells.some((c) => c.id === -3 && c.category === 'free')).toBe(true);
    expect(cells.some((c) => c.id === -2 && c.category === 'system')).toBe(true);
  });

  it('uses the allocated size when asked', () => {
    const t = tree(0, 'root', 100, [tree(1, 'a', 50, [], { alloc: 10 }), tree(2, 'b', 50, [], { alloc: 90 })]);
    const [a, b] = ['a', 'b'].map((n) => layoutTree(t, box, 'alloc').find((c) => c.name === n)!);
    expect(area(b.rect) / area(a.rect)).toBeCloseTo(9, 3);
  });

  it('drops cells too small to see and finds the deepest cell under a point', () => {
    const cells = layoutTree(root, box, 'size');
    const hit = hitTest(cells, cells.find((c) => c.id === 3)!.rect.x + 1, cells.find((c) => c.id === 3)!.rect.y + 1);
    expect(hit?.id).toBe(3);
    expect(hitTest(cells, -5, -5)).toBeNull();
    expect(layoutTree(root, { x: 0, y: 0, w: 2, h: 2 }, 'size')).toEqual([]);
  });

  it('resolves colours by category, age, top folder and extension', () => {
    const now = 1_800_000_000;
    const recent = resolveColor({ category: 'video', is_dir: false, mtime: now - 86400, ext: 'mp4', topGroup: 0 }, 'age', now);
    const ancient = resolveColor({ category: 'video', is_dir: false, mtime: now - 86400 * 2000, ext: 'mp4', topGroup: 0 }, 'age', now);
    expect(recent).not.toBe(ancient);
    expect(resolveColor({ category: 'video', is_dir: false, ext: 'mp4' }, 'extension', now)).toContain('hsl(');
    expect(resolveColor({ category: 'video', is_dir: false, topGroup: 2 }, 'folder', now)).toMatch(/^#/);
  });
});
