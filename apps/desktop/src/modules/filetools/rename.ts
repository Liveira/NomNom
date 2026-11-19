import type { RenameItem, RenameOptions } from '../../lib/ipc';

/** The pieces a pattern can use, shown as buttons under the pattern box. */
export const PATTERNS: { token: string; hint: string }[] = [
  { token: '{name}', hint: 'The old name' },
  { token: '{n}', hint: 'A number that grows: 1, 2, 3…' },
  { token: '{n:3}', hint: 'The same with zeros in front: 001, 002…' },
  { token: '{date}', hint: 'The day the file was last changed: 2026-11-19' },
  { token: '{time}', hint: 'The time it was changed: 101530' },
  { token: '{year}', hint: 'The year it was changed' },
  { token: '{month}', hint: 'The month it was changed' },
  { token: '{day}', hint: 'The day of the month it was changed' },
  { token: '{today}', hint: "Today's date" },
  { token: '{parent}', hint: "The folder's name" },
];

export function defaultOptions(): RenameOptions {
  return {
    template: '{name}',
    find: '',
    replace: '',
    case: 'keep',
    spaces: 'keep',
    order: 'given',
    start: 1,
    step: 1,
    lower_ext: false,
    today: '',
  };
}

export function countStatus(items: RenameItem[]): { ok: number; same: number; conflict: number; invalid: number } {
  const c = { ok: 0, same: 0, conflict: 0, invalid: 0 };
  for (const i of items) c[i.status] += 1;
  return c;
}

/** The last part of a path, whichever slash it uses. */
export function baseName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
