/** Turns a scan into the sentences and groups the screen shows. Pure, so it can be tested. */
import type { OrgBatch, OrgMove, OrgScan, OrgSkip, SkipCode } from '../../lib/ipc';

export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** "12 files ready · 3 left alone", or a plain sentence when there is nothing to do. */
export function headline(scan: Pick<OrgScan, 'total_moves' | 'total_skips'>): string {
  const ready = scan.total_moves;
  const left = scan.total_skips;
  if (ready === 0 && left === 0) return 'Nothing here to organize.';
  if (ready === 0) return `Nothing is ready to move. ${plural(left, 'item was', 'items were')} left alone.`;
  return `${plural(ready, 'file', 'files')} ready · ${left} left alone`;
}

export interface MoveGroup {
  category: string;
  items: OrgMove[];
}

export function groupMoves(moves: OrgMove[]): MoveGroup[] {
  const by = new Map<string, OrgMove[]>();
  for (const m of moves) by.set(m.category, [...(by.get(m.category) ?? []), m]);
  return [...by.entries()].map(([category, items]) => ({ category, items })).sort((a, b) => a.category.localeCompare(b.category));
}

const SKIP_TITLES: Record<SkipCode, string> = {
  software_folder: 'Folders that look like software',
  folder_not_opened: 'Folders (not opened)',
  program: 'Programs and scripts',
  archive: 'Archives',
  link: 'Links to other places',
  hidden: 'Hidden or system items',
  cloud: 'Files stored in the cloud',
  in_progress: 'Still downloading',
  recently_changed: 'Changed a moment ago',
  locked: 'In use by another program',
  unknown: "Types NomNom doesn't sort",
  mismatch: "Contents don't match the file type",
  empty: 'Empty files',
  conflict: 'Name already taken',
  cross_volume: 'On another drive',
  already_organized: 'Already organized',
  unreadable: "Couldn't be read",
  excluded: 'Left out by your choice',
  protected: 'Not allowed',
  too_many: 'Too many items to look at',
};

/** Shown in this order: the reasons that matter most for safety come first. */
const SKIP_ORDER: SkipCode[] = [
  'software_folder', 'program', 'archive', 'link', 'folder_not_opened', 'mismatch', 'conflict', 'locked',
  'in_progress', 'recently_changed', 'hidden', 'cloud', 'unknown', 'empty', 'cross_volume', 'excluded',
  'already_organized', 'unreadable', 'protected', 'too_many',
];

export interface SkipGroup {
  code: SkipCode;
  title: string;
  items: OrgSkip[];
}

export function groupSkips(skips: OrgSkip[]): SkipGroup[] {
  const by = new Map<SkipCode, OrgSkip[]>();
  for (const s of skips) by.set(s.code, [...(by.get(s.code) ?? []), s]);
  return [...by.entries()]
    .map(([code, items]) => ({ code, title: SKIP_TITLES[code] ?? 'Other', items }))
    .sort((a, b) => SKIP_ORDER.indexOf(a.code) - SKIP_ORDER.indexOf(b.code));
}

/** What a run did, in one sentence. */
export function resultLine(r: { moved: number; failed: { length: number }; stopped: string | null }): string {
  const parts = [`Moved ${plural(r.moved, 'file', 'files')}.`];
  if (r.failed.length > 0) parts.push(`${plural(r.failed.length, 'file was', 'files were')} left where ${r.failed.length === 1 ? 'it was' : 'they were'}.`);
  if (r.stopped) parts.push(r.stopped);
  return parts.join(' ');
}

export function undoLine(u: { restored: number; skipped: { length: number } }): string {
  const parts = [`Put back ${plural(u.restored, 'file', 'files')}.`];
  if (u.skipped.length > 0) parts.push(`${plural(u.skipped.length, 'file was', 'files were')} left where ${u.skipped.length === 1 ? 'it is' : 'they are'} now.`);
  return parts.join(' ');
}

/** A batch can be undone while any of its files are still where they were moved. */
export function canUndo(b: Pick<OrgBatch, 'moved'>): boolean {
  return b.moved > 0;
}

export function batchTitle(b: OrgBatch): string {
  const what = b.mode === 'auto' ? 'Automatic' : 'Manual';
  const n = Math.max(b.moved, b.undone);
  return `${what} · ${plural(n, 'file', 'files')}`;
}

export function batchStatus(b: OrgBatch): string {
  if (b.moved === 0 && b.undone > 0) return 'Put back';
  if (b.undone > 0) return `${b.undone} put back, ${b.moved} still moved`;
  if (b.failed > 0) return `${b.failed} left where ${b.failed === 1 ? 'it was' : 'they were'}`;
  return 'Done';
}
