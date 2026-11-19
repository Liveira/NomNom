/** The geometry behind the screenshot editor: no drawing here, only the numbers. */

export interface Pt {
  x: number;
  y: number;
}
export interface Box {
  x: number;
  y: number;
  w: number;
  h: number;
}

export type Shape =
  | { kind: 'pen'; color: string; width: number; points: Pt[] }
  | { kind: 'arrow'; color: string; width: number; from: Pt; to: Pt }
  | { kind: 'rect'; color: string; width: number; box: Box }
  | { kind: 'mark'; color: string; box: Box }
  | { kind: 'blur'; box: Box }
  | { kind: 'step'; color: string; width: number; at: Pt; n: number }
  | { kind: 'text'; color: string; width: number; at: Pt; text: string };

/** The rectangle between two corners, whichever way it was dragged. */
export function boxOf(a: Pt, b: Pt): Box {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(b.x - a.x), h: Math.abs(b.y - a.y) };
}

/** Keeps a box inside a picture of `w` by `h`. */
export function clampBox(b: Box, w: number, h: number): Box {
  const x = Math.max(0, Math.min(b.x, w));
  const y = Math.max(0, Math.min(b.y, h));
  return { x, y, w: Math.max(0, Math.min(b.x + b.w, w) - x), h: Math.max(0, Math.min(b.y + b.h, h) - y) };
}

/**
 * The line of an arrow and its head. The line stops at the base of the head, so a thick line doesn't
 * poke out past the point. `null` for an arrow too short to draw.
 */
export function arrowParts(from: Pt, to: Pt, width: number): { base: Pt; left: Pt; right: Pt; tip: Pt } | null {
  const dx = to.x - from.x;
  const dy = to.y - from.y;
  const len = Math.hypot(dx, dy);
  if (len < 4) return null;
  const head = Math.min(len * 0.6, Math.max(14, width * 4.5));
  const ux = dx / len;
  const uy = dy / len;
  const base = { x: to.x - ux * head * 0.85, y: to.y - uy * head * 0.85 };
  const half = head * 0.5;
  return {
    base,
    left: { x: base.x - uy * half, y: base.y + ux * half },
    right: { x: base.x + uy * half, y: base.y - ux * half },
    tip: to,
  };
}

/** The size of the blocks that hide a part of the picture: big enough that nothing can be read. */
export function blockSize(b: Box): number {
  return Math.max(8, Math.round(Math.min(b.w, b.h) / 8));
}

/** The number for the next step marker: one more than the highest so far. */
export function nextStep(shapes: Shape[]): number {
  return shapes.reduce((n, s) => (s.kind === 'step' ? Math.max(n, s.n) : n), 0) + 1;
}

/** Font size for text and the radius of a step marker, from the chosen line width. */
export const textSize = (width: number) => 14 + width * 4;
export const stepRadius = (width: number) => 11 + width * 2.5;

export const COLORS = ['#ff4d4f', '#ffb020', '#2ecc71', '#2b9cff', '#a259ff', '#ffffff', '#111111'];
export const WIDTHS = [2, 4, 7];

/** Is a box worth keeping, or only a stray click? */
export const isReal = (b: Box) => b.w >= 4 && b.h >= 4;
