import type { CompareRow } from '../../lib/ipc';

export type Direction = 'to_right' | 'to_left' | 'both';
export type Action = 'copy_right' | 'copy_left';

/** What a sync in this direction does for a row. The same rules as in NomNom's core: it only ever copies. */
export function suggest(row: Pick<CompareRow, 'status'>, dir: Direction): Action | null {
  switch (row.status) {
    case 'only_left':
    case 'left_newer':
      return dir === 'to_right' || dir === 'both' ? 'copy_right' : null;
    case 'only_right':
    case 'right_newer':
      return dir === 'to_left' || dir === 'both' ? 'copy_left' : null;
    default:
      return null;
  }
}

/** Does this copy replace a file that is already there? */
export function replaces(row: Pick<CompareRow, 'left' | 'right'>, action: Action): boolean {
  return action === 'copy_right' ? row.right !== null : row.left !== null;
}

export const STATUS_LABEL: Record<CompareRow['status'], string> = {
  only_left: 'Only on the left',
  only_right: 'Only on the right',
  same: 'The same',
  left_newer: 'Newer on the left',
  right_newer: 'Newer on the right',
  different: 'Different, same time',
};

export type Filter = 'changes' | 'only_left' | 'only_right' | 'different' | 'same';

export function matchesFilter(row: Pick<CompareRow, 'status'>, f: Filter): boolean {
  switch (f) {
    case 'changes':
      return row.status !== 'same';
    case 'only_left':
      return row.status === 'only_left';
    case 'only_right':
      return row.status === 'only_right';
    case 'different':
      return row.status === 'left_newer' || row.status === 'right_newer' || row.status === 'different';
    default:
      return row.status === 'same';
  }
}

/** Counts per status, for the filter buttons. */
export function counts(rows: Pick<CompareRow, 'status'>[]): Record<Filter, number> {
  const out: Record<Filter, number> = { changes: 0, only_left: 0, only_right: 0, different: 0, same: 0 };
  for (const r of rows) (Object.keys(out) as Filter[]).forEach((f) => matchesFilter(r, f) && out[f]++);
  return out;
}

/** The part of a path after the last slash. */
export const leaf = (rel: string) => rel.slice(rel.lastIndexOf('/') + 1);
