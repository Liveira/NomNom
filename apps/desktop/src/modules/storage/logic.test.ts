import { describe, expect, it } from 'vitest';
import type { StProgress } from '../../lib/ipc';
import {
  completenessNote, defaultColumnState, driveKindLabel, formatElapsed, moveColumn, nextSelection,
  parseColumnState, parseSize, usagePercent, visibleColumns,
} from './logic';

const prog = (p: Partial<StProgress>): StProgress => ({
  id: 1, root: 'C:\\', state: 'done', files: 10, dirs: 2, bytes: 1000, errors: 0, elapsed_ms: 1000,
  drive_used: 1000, drive_total: 5000, ...p,
});

describe('numbers', () => {
  it('keeps the usage percentage inside 0 to 100', () => {
    expect(usagePercent(50, 200)).toBe(25);
    expect(usagePercent(300, 200)).toBe(100);
    expect(usagePercent(5, 0)).toBe(0);
  });
  it('formats elapsed time', () => {
    expect(formatElapsed(65_000)).toBe('1:05');
    expect(formatElapsed(3_725_000)).toBe('1:02:05');
  });
  it('reads typed sizes', () => {
    expect(parseSize('', 'MB')).toBeNull();
    expect(parseSize('1,5', 'GB')).toBe(1.5 * 1024 ** 3);
    expect(parseSize('abc', 'MB')).toBe('bad');
    expect(parseSize('-1', 'MB')).toBe('bad');
  });
  it('names drive kinds', () => {
    expect(driveKindLabel({ kind: 'fixed', media: 'ssd' })).toBe('SSD');
    expect(driveKindLabel({ kind: 'network', media: 'unknown' })).toBe('Network drive');
  });
});

describe('completeness note', () => {
  it('says nothing while scanning or for a folder scan', () => {
    expect(completenessNote(prog({ state: 'running' }), true)).toBeNull();
    expect(completenessNote(prog({}), false)).toBeNull();
  });
  it('gives the real numbers and no excuse when they match', () => {
    const n = completenessNote(prog({ bytes: 1000, drive_used: 1010 }), true)!;
    expect(n).toContain('Windows reports');
    expect(n).not.toContain('difference');
  });
  it('explains a gap without inventing one', () => {
    const n = completenessNote(prog({ bytes: 10 * 1024 ** 3, drive_used: 20 * 1024 ** 3, errors: 3 }), true)!;
    expect(n).toContain('10.0 GB');
    expect(n).toContain('20.0 GB');
    expect(n).toContain('page file');
    expect(n).toContain('3 places');
  });
  it('says a stopped scan is partial', () => {
    expect(completenessNote(prog({ state: 'cancelled' }), true)).toContain('partial');
  });
});

describe('columns', () => {
  it('always shows Name, hides what is hidden and keeps a minimum width', () => {
    const s = { ...defaultColumnState(), hidden: ['name', 'files'], widths: { size: 10 } };
    const cols = visibleColumns(s);
    expect(cols.map((c) => c.key)).toEqual(['name', 'size', 'alloc', 'percent', 'dirs', 'modified']);
    expect(cols.find((c) => c.key === 'size')!.w).toBe(50);
  });
  it('reorders and survives damaged saved state', () => {
    expect(moveColumn(['a', 'b', 'c'], 'c', 'a')).toEqual(['c', 'a', 'b']);
    expect(moveColumn(['a', 'b'], 'x', 'a')).toEqual(['a', 'b']);
    expect(parseColumnState('not json')).toEqual(defaultColumnState());
    const s = parseColumnState(JSON.stringify({ order: ['size', 'bogus'], hidden: ['dirs'], widths: { size: 200 } }));
    expect(visibleColumns(s).map((c) => c.key).slice(0, 2)).toEqual(['size', 'name']);
  });
});

describe('selection', () => {
  const ids = [10, 11, 12, 13, 14];
  const none = new Set<number>();
  it('a plain click selects one', () => {
    expect(nextSelection(new Set([10, 11]), 10, ids, 3, { ctrl: false, shift: false })).toEqual({ selected: new Set([13]), anchor: 13 });
  });
  it('Ctrl toggles', () => {
    const a = nextSelection(none, null, ids, 1, { ctrl: true, shift: false });
    const b = nextSelection(a.selected, a.anchor, ids, 3, { ctrl: true, shift: false });
    expect([...b.selected].sort()).toEqual([11, 13]);
    expect(nextSelection(b.selected, b.anchor, ids, 1, { ctrl: true, shift: false }).selected).toEqual(new Set([13]));
  });
  it('Shift selects a range from the anchor, in either direction', () => {
    expect(nextSelection(new Set([11]), 11, ids, 3, { ctrl: false, shift: true }).selected).toEqual(new Set([11, 12, 13]));
    expect(nextSelection(new Set([13]), 13, ids, 1, { ctrl: false, shift: true }).selected).toEqual(new Set([11, 12, 13]));
  });
  it('ignores a click past the end', () => {
    expect(nextSelection(none, null, ids, 9, { ctrl: false, shift: false }).selected).toBe(none);
  });
});

import { attributeLabels, shortcutAction, shortcutsActive } from './logic';

describe('shortcuts', () => {
  const k = (key: string, m: Partial<{ ctrl: boolean; shift: boolean; alt: boolean }> = {}) => shortcutAction({ key, ctrl: false, shift: false, alt: false, ...m });
  it('maps the documented keys', () => {
    expect(k('Delete')).toBe('recycle');
    expect(k('Delete', { shift: true })).toBe('delete');
    expect(k('c', { ctrl: true })).toBe('copy');
    expect(k('X', { ctrl: true })).toBe('cut');
    expect(k('v', { ctrl: true })).toBe('paste');
    expect(k('a', { ctrl: true })).toBe('selectAll');
    expect(k('F2')).toBe('rename');
    expect(k('F5')).toBe('refresh');
    expect(k('Enter', { alt: true })).toBe('properties');
  });
  it('ignores combinations that mean something else', () => {
    expect(k('Delete', { ctrl: true })).toBeNull();
    expect(k('Delete', { shift: true, ctrl: true })).toBeNull();
    expect(k('c')).toBeNull();
    expect(k('c', { ctrl: true, shift: true })).toBeNull();
    expect(k('Enter')).toBeNull();
    expect(k('F2', { shift: true })).toBeNull();
  });
  it('only act on the file list, never while typing, in a dialog or during a scan', () => {
    const base = { dialogOpen: false, menuOpen: false, targetIsField: false, tab: 'explorer', hasSession: true, busy: false };
    expect(shortcutsActive(base)).toBe(true);
    for (const off of [{ dialogOpen: true }, { menuOpen: true }, { targetIsField: true }, { tab: 'rankings' }, { hasSession: false }, { busy: true }]) {
      expect(shortcutsActive({ ...base, ...off })).toBe(false);
    }
  });
  it('describes attributes in words', () => {
    expect(attributeLabels(0x1 | 0x2, 0)).toEqual(['Read-only', 'Hidden']);
    expect(attributeLabels(0, 2 | 4)).toEqual(['Link (not followed)', 'Same data as another name']);
  });
});
