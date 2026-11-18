/** Squarified treemap layout and colour palettes. Pure: takes sizes and a rectangle, returns rectangles. */
import type { StTree } from '../../lib/ipc';

export interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface Placed<T> {
  item: T;
  rect: Rect;
}

function worst(row: number[], side: number): number {
  const sum = row.reduce((a, b) => a + b, 0);
  const max = Math.max(...row);
  const min = Math.min(...row);
  const s2 = side * side;
  return Math.max((s2 * max) / (sum * sum), (sum * sum) / (s2 * min));
}

/**
 * Lays `items` out inside `rect` so that each area is proportional to its value and the shapes are
 * as square as possible. Items with no value are left out.
 */
export function squarify<T>(items: T[], value: (t: T) => number, rect: Rect): Placed<T>[] {
  const list = items.filter((i) => value(i) > 0).sort((a, b) => value(b) - value(a));
  const total = list.reduce((s, i) => s + value(i), 0);
  if (list.length === 0 || total <= 0 || rect.w <= 0 || rect.h <= 0) return [];
  const scale = (rect.w * rect.h) / total;
  const areas = list.map((i) => value(i) * scale);
  const out: Placed<T>[] = [];
  let { x, y, w, h } = rect;
  let start = 0;
  while (start < list.length) {
    const side = Math.min(w, h);
    let end = start + 1;
    while (end < list.length && worst(areas.slice(start, end + 1), side) <= worst(areas.slice(start, end), side)) end++;
    const row = areas.slice(start, end);
    const rowSum = row.reduce((a, b) => a + b, 0);
    if (w >= h) {
      const colW = rowSum / h;
      let cy = y;
      row.forEach((a, k) => {
        const ih = a / colW;
        out.push({ item: list[start + k], rect: { x, y: cy, w: colW, h: ih } });
        cy += ih;
      });
      x += colW;
      w -= colW;
    } else {
      const rowH = rowSum / w;
      let cx = x;
      row.forEach((a, k) => {
        const iw = a / rowH;
        out.push({ item: list[start + k], rect: { x: cx, y, w: iw, h: rowH } });
        cx += iw;
      });
      y += rowH;
      h -= rowH;
    }
    start = end;
  }
  return out;
}

export type Metric = 'size' | 'alloc';
export type ColorMode = 'category' | 'age' | 'folder' | 'extension';

export interface Cell {
  id: number;
  name: string;
  is_dir: boolean;
  category: string;
  ext: string;
  mtime: number | null;
  topGroup: number;
  value: number;
  rect: Rect;
  depth: number;
  /** True for the grey "N smaller items" block. */
  folded: boolean;
  foldedCount: number;
  parent: number | null;
}

const HEADER = 16;
const PAD = 2;

export interface ExtraSpace {
  freeBytes?: number;
  systemBytes?: number;
}

/**
 * Lays out a whole nested tree. Folders get a small label strip and their children fill the rest.
 * Cells too small to see are dropped (their size stays inside the parent's area).
 */
export function layoutTree(root: StTree, rect: Rect, metric: Metric, minSide = 3, extra?: ExtraSpace): Cell[] {
  const cells: Cell[] = [];
  const val = (n: { size: number; alloc: number }) => (metric === 'size' ? n.size : n.alloc);

  const place = (node: StTree, r: Rect, depth: number, parent: number | null, topGroup: number) => {
    if (r.w < minSide || r.h < minSide) return;
    let totalVal = val(node);
    if (depth === 0 && extra) {
      totalVal += (extra.freeBytes ?? 0) + (extra.systemBytes ?? 0);
    }
    cells.push({
      id: node.id,
      name: node.name,
      is_dir: node.is_dir,
      category: node.category,
      ext: node.ext ?? '',
      mtime: node.mtime ?? null,
      topGroup,
      value: totalVal,
      rect: r,
      depth,
      folded: false,
      foldedCount: 0,
      parent,
    });
    if (!node.is_dir) return;
    type K = StTree & { _folded?: boolean; _virtual?: 'free' | 'system' };
    const kids: K[] = [...node.children];
    const hiddenValue = metric === 'size' ? node.hidden_size : node.hidden_alloc;
    if (node.hidden_count > 0 && hiddenValue > 0) {
      kids.push({
        id: -1,
        name: `${node.hidden_count} smaller items`,
        is_dir: false,
        size: node.hidden_size,
        alloc: node.hidden_alloc,
        category: 'other',
        children: [],
        hidden_count: 0,
        hidden_size: 0,
        hidden_alloc: 0,
        _folded: true,
      });
    }
    if (depth === 0 && extra) {
      if (extra.systemBytes && extra.systemBytes > 0) {
        kids.push({
          id: -2,
          name: '[System and unreadable]',
          is_dir: false,
          size: extra.systemBytes,
          alloc: extra.systemBytes,
          category: 'system',
          children: [],
          hidden_count: 0,
          hidden_size: 0,
          hidden_alloc: 0,
          _virtual: 'system',
        });
      }
      if (extra.freeBytes && extra.freeBytes > 0) {
        kids.push({
          id: -3,
          name: '[Free space]',
          is_dir: false,
          size: extra.freeBytes,
          alloc: extra.freeBytes,
          category: 'free',
          children: [],
          hidden_count: 0,
          hidden_size: 0,
          hidden_alloc: 0,
          _virtual: 'free',
        });
      }
    }
    if (kids.length === 0) return;
    const hasHeader = r.h > HEADER * 2 && r.w > 40;
    const inner: Rect = {
      x: r.x + PAD,
      y: r.y + (hasHeader ? HEADER : PAD),
      w: r.w - PAD * 2,
      h: r.h - (hasHeader ? HEADER : PAD) - PAD,
    };
    let groupIdx = 0;
    for (const p of squarify(kids, (k) => val(k), inner)) {
      const nextGroup = depth === 0 ? groupIdx++ : topGroup;
      if (p.item._folded) {
        if (p.rect.w >= minSide && p.rect.h >= minSide) {
          cells.push({
            id: -1,
            name: p.item.name,
            is_dir: false,
            category: 'folded',
            ext: '',
            mtime: null,
            topGroup: nextGroup,
            value: val(p.item),
            rect: p.rect,
            depth: depth + 1,
            folded: true,
            foldedCount: node.hidden_count,
            parent: node.id,
          });
        }
      } else if (p.item._virtual) {
        if (p.rect.w >= minSide && p.rect.h >= minSide) {
          cells.push({
            id: p.item.id,
            name: p.item.name,
            is_dir: false,
            category: p.item.category,
            ext: '',
            mtime: null,
            topGroup: nextGroup,
            value: val(p.item),
            rect: p.rect,
            depth: depth + 1,
            folded: false,
            foldedCount: 0,
            parent: node.id,
          });
        }
      } else {
        place(p.item, p.rect, depth + 1, node.id, nextGroup);
      }
    }
  };
  place(root, rect, 0, null, 0);
  return cells;
}

/** The deepest cell under a point (cells are in paint order, parents before children). */
export function hitTest(cells: Cell[], x: number, y: number): Cell | null {
  let hit: Cell | null = null;
  for (const c of cells) {
    const r = c.rect;
    if (x >= r.x && x < r.x + r.w && y >= r.y && y < r.y + r.h) hit = c;
  }
  return hit;
}

export const CATEGORY_COLORS: Record<string, string> = {
  video: '#e5737d',
  image: '#6fb7ff',
  audio: '#b48cf2',
  document: '#7ed3a0',
  executable: '#f0b450',
  archive: '#ef8f5b',
  temp: '#9aa3ad',
  other: '#c3b1c0',
  folder: '#8f7aa8',
  folded: '#6d6477',
  free: '#5d7c6b',
  system: '#8f7a68',
};

export const CATEGORY_LABELS: Record<string, string> = {
  video: 'Video',
  image: 'Pictures',
  audio: 'Sound',
  document: 'Documents',
  executable: 'Programs',
  archive: 'Archives',
  temp: 'Temporary',
  other: 'Other',
  folder: 'Folders',
};

export const AGE_PALETTE: { label: string; maxDays: number; color: string }[] = [
  { label: 'Last 7 days', maxDays: 7, color: '#ff8566' },
  { label: '8 to 30 days', maxDays: 30, color: '#ffb070' },
  { label: '31 to 90 days', maxDays: 90, color: '#e6cc6e' },
  { label: '3m to 1 year', maxDays: 365, color: '#7ed3a0' },
  { label: '1 to 3 years', maxDays: 1095, color: '#6fb7ff' },
  { label: 'Over 3 years', maxDays: Infinity, color: '#7d8597' },
];

export const FOLDER_PALETTE = [
  '#f09a55', '#6fb7ff', '#7ed3a0', '#e5737d', '#b48cf2',
  '#e6cc6e', '#5ec2c6', '#ef8f5b', '#e887b2', '#9ab87a',
];

export function extensionColor(ext: string): string {
  if (!ext) return '#9aa3ad';
  let h = 2166136261;
  for (let i = 0; i < ext.length; i++) {
    h ^= ext.charCodeAt(i);
    h = Math.imul(h, 16777619);
  }
  const hue = Math.abs(h) % 360;
  return `hsl(${hue}, 62%, 67%)`;
}

export function resolveColor(
  item: { category: string; is_dir: boolean; mtime?: number | null; topGroup?: number; ext?: string; folded?: boolean },
  mode: ColorMode,
  nowUnix = Math.floor(Date.now() / 1000),
): string {
  if (item.folded) return CATEGORY_COLORS.folded;
  if (item.category === 'free') return CATEGORY_COLORS.free;
  if (item.category === 'system') return CATEGORY_COLORS.system;
  if (mode === 'category') return CATEGORY_COLORS[item.category] ?? CATEGORY_COLORS.other;
  if (mode === 'folder') {
    const idx = Math.abs(item.topGroup ?? 0) % FOLDER_PALETTE.length;
    return FOLDER_PALETTE[idx];
  }
  if (mode === 'extension') {
    if (item.is_dir) return CATEGORY_COLORS.folder;
    return extensionColor(item.ext ?? '');
  }
  // mode === 'age'
  if (item.mtime == null) return '#8b8394';
  const days = Math.max(0, (nowUnix - item.mtime) / 86400);
  for (const b of AGE_PALETTE) {
    if (days <= b.maxDays) return b.color;
  }
  return AGE_PALETTE[AGE_PALETTE.length - 1].color;
}
