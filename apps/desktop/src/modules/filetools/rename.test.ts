import { describe, expect, it } from 'vitest';
import type { RenameItem } from '../../lib/ipc';
import { PATTERNS, baseName, countStatus, defaultOptions } from './rename';

const item = (status: RenameItem['status']): RenameItem => ({ from: '/a', from_name: 'a', to_name: 'b', status, note: '' });

describe('rename helpers', () => {
  it('counts what the preview says', () => {
    expect(countStatus([item('ok'), item('ok'), item('same'), item('conflict'), item('invalid')])).toEqual({ ok: 2, same: 1, conflict: 1, invalid: 1 });
    expect(countStatus([])).toEqual({ ok: 0, same: 0, conflict: 0, invalid: 0 });
  });
  it('starts with the old name, so nothing changes until the person asks', () => {
    expect(defaultOptions().template).toBe('{name}');
    expect(defaultOptions().case).toBe('keep');
  });
  it('takes the file name from either kind of path', () => {
    expect(baseName('C:\\Users\\me\\a.txt')).toBe('a.txt');
    expect(baseName('/home/me/a.txt')).toBe('a.txt');
  });
  it('offers each piece once, with a hint', () => {
    expect(new Set(PATTERNS.map((p) => p.token)).size).toBe(PATTERNS.length);
    expect(PATTERNS.every((p) => p.hint.length > 5)).toBe(true);
  });
});
