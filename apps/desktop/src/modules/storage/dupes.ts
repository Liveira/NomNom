/** Choosing which duplicate copies to remove. Pure: the rules that keep one copy safe live here and in the app. */
import type { StDupGroup } from '../../lib/ipc';

/** For each group, the copy that stays by default: the oldest, then the shortest path. */
export function defaultKeep(g: StDupGroup): number {
  const sorted = [...g.copies].sort((a, b) => (a.mtime ?? Infinity) - (b.mtime ?? Infinity) || a.path.length - b.path.length || a.path.localeCompare(b.path));
  return sorted[0].node;
}

export interface Choice {
  keep: number;
  /** Copies marked for removal. Never includes `keep`. */
  remove: Set<number>;
}

export function initialChoice(g: StDupGroup): Choice {
  const keep = defaultKeep(g);
  return { keep, remove: new Set(g.copies.filter((c) => c.node !== keep).map((c) => c.node)) };
}

/** Makes `node` the copy that stays. It can't be marked for removal at the same time. */
export function setKeep(c: Choice, node: number): Choice {
  const remove = new Set(c.remove);
  remove.delete(node);
  return { keep: node, remove };
}

export function toggleRemove(c: Choice, node: number): Choice {
  if (node === c.keep) return c; // the copy that stays can't be removed
  const remove = new Set(c.remove);
  if (remove.has(node)) remove.delete(node);
  else remove.add(node);
  return { keep: c.keep, remove };
}

/** What would be removed and how much space that gives back. A hardlink name frees nothing by itself. */
export function summarize(groups: StDupGroup[], choices: Map<number, Choice>): { files: number; bytes: number; groups: number } {
  let files = 0;
  let bytes = 0;
  let touched = 0;
  groups.forEach((g, i) => {
    const c = choices.get(i);
    if (!c) return;
    const n = g.copies.filter((x) => c.remove.has(x.node)).length;
    if (n > 0) touched++;
    files += n;
    bytes += g.copies.filter((x) => c.remove.has(x.node) && x.names === 1).length * g.size;
  });
  return { files, bytes, groups: touched };
}

/** The request for the app: one entry per group that has something to remove. */
export function toRequest(groups: StDupGroup[], choices: Map<number, Choice>): { keep: number; remove: number[] }[] {
  const out: { keep: number; remove: number[] }[] = [];
  groups.forEach((g, i) => {
    const c = choices.get(i);
    if (!c) return;
    const remove = g.copies.filter((x) => c.remove.has(x.node) && x.node !== c.keep).map((x) => x.node);
    if (remove.length > 0) out.push({ keep: c.keep, remove });
  });
  return out;
}
