/** Geometry for the pie and multi-level sunburst ring charts. Pure, so the arcs can be checked. */
import type { StTree } from '../../lib/ipc';
import type { Metric } from './treemap';

export interface Slice {
  key: string;
  value: number;
  /** Share of the whole, 0 to 1. */
  share: number;
  path: string;
}

/** SVG paths for a pie of radius 1 centred on the origin, starting at 12 o'clock, clockwise. */
export function pieSlices(items: { key: string; value: number }[]): Slice[] {
  const list = items.filter((i) => i.value > 0);
  const total = list.reduce((s, i) => s + i.value, 0);
  if (total <= 0) return [];
  let angle = -Math.PI / 2;
  return list.map((i) => {
    const share = i.value / total;
    const sweep = share * Math.PI * 2;
    const a0 = angle;
    const a1 = angle + sweep;
    angle = a1;
    const p = (a: number) => `${Math.cos(a).toFixed(5)} ${Math.sin(a).toFixed(5)}`;
    const path =
      share >= 0.99999
        ? `M 0 -1 A 1 1 0 1 1 0 1 A 1 1 0 1 1 0 -1 Z`
        : `M 0 0 L ${p(a0)} A 1 1 0 ${sweep > Math.PI ? 1 : 0} 1 ${p(a1)} Z`;
    return { key: i.key, value: i.value, share, path };
  });
}

export interface RingArc {
  id: number;
  name: string;
  is_dir: boolean;
  category: string;
  ext: string;
  mtime: number | null;
  topGroup: number;
  depth: number;
  value: number;
  share: number;
  folded: boolean;
  path: string;
}

function donutArcPath(r0: number, r1: number, a0: number, a1: number): string {
  const sweep = a1 - a0;
  if (sweep >= Math.PI * 2 - 1e-4) {
    const mid = a0 + Math.PI;
    return `${donutArcPath(r0, r1, a0, mid)} ${donutArcPath(r0, r1, mid, a0 + Math.PI * 2 - 1e-5)}`;
  }
  const c0 = Math.cos(a0);
  const s0 = Math.sin(a0);
  const c1 = Math.cos(a1);
  const s1 = Math.sin(a1);
  const large = sweep > Math.PI ? 1 : 0;
  const f = (n: number) => n.toFixed(5);
  return `M ${f(r1 * c0)} ${f(r1 * s0)} A ${f(r1)} ${f(r1)} 0 ${large} 1 ${f(r1 * c1)} ${f(r1 * s1)} L ${f(r0 * c1)} ${f(r0 * s1)} A ${f(r0)} ${f(r0)} 0 ${large} 0 ${f(r0 * c0)} ${f(r0 * s0)} Z`;
}

/**
 * Builds concentric ring arcs for a hierarchical sunburst chart inside radius `0.26..1.0`.
 * Arcs narrower than `minSweep` radians are omitted so thousands of tiny files don't clutter SVG.
 */
export function sunburstArcs(root: StTree, metric: Metric, maxDepth = 4, minSweep = 0.008): RingArc[] {
  const val = (n: { size: number; alloc: number }) => (metric === 'size' ? n.size : n.alloc);
  const total = val(root);
  if (total <= 0 || maxDepth < 1) return [];

  const innerR = 0.26;
  const ringW = (0.98 - innerR) / maxDepth;
  const arcs: RingArc[] = [];

  const visit = (node: StTree, depth: number, aStart: number, aEnd: number, topGroup: number) => {
    if (depth >= maxDepth || !node.is_dir) return;
    type Child = StTree & { _folded?: boolean };
    const kids: Child[] = [...node.children];
    const hiddenVal = metric === 'size' ? node.hidden_size : node.hidden_alloc;
    if (node.hidden_count > 0 && hiddenVal > 0) {
      kids.push({
        id: -1,
        name: `${node.hidden_count} smaller items`,
        is_dir: false,
        size: node.hidden_size,
        alloc: node.hidden_alloc,
        category: 'folded',
        children: [],
        hidden_count: 0,
        hidden_size: 0,
        hidden_alloc: 0,
        _folded: true,
      });
    }
    const parentVal = Math.max(val(node), kids.reduce((s, k) => s + val(k), 0));
    if (parentVal <= 0) return;

    const span = aEnd - aStart;
    const r0 = innerR + depth * ringW + 0.006;
    const r1 = innerR + (depth + 1) * ringW - 0.004;
    let cursor = aStart;
    let groupIdx = 0;

    for (const k of kids) {
      const kv = val(k);
      if (kv <= 0) continue;
      const sweep = (kv / parentVal) * span;
      const next = cursor + sweep;
      const grp = depth === 0 ? groupIdx++ : topGroup;
      if (sweep >= minSweep) {
        arcs.push({
          id: k.id,
          name: k.name,
          is_dir: k.is_dir,
          category: k._folded ? 'folded' : k.category,
          ext: k.ext ?? '',
          mtime: k.mtime ?? null,
          topGroup: grp,
          depth: depth + 1,
          value: kv,
          share: kv / total,
          folded: !!k._folded,
          path: donutArcPath(r0, r1, cursor, next),
        });
        if (!k._folded && k.is_dir && k.children.length > 0) {
          visit(k, depth + 1, cursor, next, grp);
        }
      }
      cursor = next;
    }
  };

  visit(root, 0, -Math.PI / 2, Math.PI * 1.5, 0);
  return arcs;
}
