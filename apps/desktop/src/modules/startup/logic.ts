import type { StartupEffect, StartupItem } from '../../lib/ipc';

export type Impact = 'high' | 'medium' | 'low' | 'none';

/** "4.5 s", "26 s" or "1 min 12 s" for milliseconds. */
export function delayText(ms: number): string {
  const s = ms / 1000;
  if (s < 10) return `${s.toFixed(1)} s`;
  if (s < 60) return `${Math.round(s)} s`;
  const total = Math.round(s);
  return `${Math.floor(total / 60)} min ${total % 60} s`;
}

/** How much a program slowed the start-ups, by its average: 15 s or more is a lot, 5 s a little. */
export function impactOf(effect: StartupEffect | null): Impact {
  if (!effect || effect.times === 0) return 'none';
  const avg = effect.total_ms / effect.times / 1000;
  if (avg >= 15) return 'high';
  if (avg >= 5) return 'medium';
  return 'low';
}

const order: Record<Impact, number> = { high: 0, medium: 1, low: 2, none: 3 };

/** Turned-on entries first, and among them the ones that slow the start-up most. */
export function sortItems(items: StartupItem[]): StartupItem[] {
  return [...items].sort((a, b) => {
    if (a.enabled !== b.enabled) return a.enabled ? -1 : 1;
    const byImpact = order[impactOf(a.effect)] - order[impactOf(b.effect)];
    if (byImpact !== 0) return byImpact;
    const worse = (b.effect?.total_ms ?? 0) / Math.max(1, b.effect?.times ?? 1) - (a.effect?.total_ms ?? 0) / Math.max(1, a.effect?.times ?? 1);
    return worse !== 0 ? worse : a.name.localeCompare(b.name);
  });
}
