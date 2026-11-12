/** Geometry for the pie chart. Pure, so the arcs can be checked. */

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
    // A full circle can't be drawn as one arc; split it in two halves.
    const path =
      share >= 0.99999
        ? `M 0 -1 A 1 1 0 1 1 0 1 A 1 1 0 1 1 0 -1 Z`
        : `M 0 0 L ${p(a0)} A 1 1 0 ${sweep > Math.PI ? 1 : 0} 1 ${p(a1)} Z`;
    return { key: i.key, value: i.value, share, path };
  });
}
