import { describe, expect, it } from 'vitest';
import type { CompareRow } from '../../lib/ipc';
import { counts, leaf, matchesFilter, replaces, suggest } from './logic';

const row = (status: CompareRow['status'], left = true, right = true): CompareRow => ({
  rel: 'a/b.txt',
  status,
  left: left ? [1, 10] : null,
  right: right ? [1, 5] : null,
});

describe('suggest', () => {
  it('copies only toward the side that is missing the file or has an older one', () => {
    expect(suggest(row('only_left'), 'to_right')).toBe('copy_right');
    expect(suggest(row('only_left'), 'to_left')).toBeNull();
    expect(suggest(row('only_right'), 'to_left')).toBe('copy_left');
    expect(suggest(row('left_newer'), 'both')).toBe('copy_right');
    expect(suggest(row('right_newer'), 'to_right')).toBeNull();
    expect(suggest(row('right_newer'), 'both')).toBe('copy_left');
  });
  it('leaves equal and undecidable files alone', () => {
    for (const d of ['to_right', 'to_left', 'both'] as const) {
      expect(suggest(row('same'), d)).toBeNull();
      expect(suggest(row('different'), d)).toBeNull();
    }
  });
});

describe('replaces', () => {
  it('is true only when the target already has the file', () => {
    expect(replaces(row('left_newer'), 'copy_right')).toBe(true);
    expect(replaces(row('only_left', true, false), 'copy_right')).toBe(false);
    expect(replaces(row('only_right', false, true), 'copy_left')).toBe(false);
  });
});

describe('filters', () => {
  it('sorts rows into the groups of buttons', () => {
    const rows = [row('same'), row('only_left'), row('only_right'), row('left_newer'), row('different')];
    expect(counts(rows)).toEqual({ changes: 4, only_left: 1, only_right: 1, different: 2, same: 1 });
    expect(matchesFilter(row('right_newer'), 'different')).toBe(true);
    expect(matchesFilter(row('same'), 'changes')).toBe(false);
  });
  it('takes the file name from a path', () => {
    expect(leaf('a/b/c.txt')).toBe('c.txt');
    expect(leaf('c.txt')).toBe('c.txt');
  });
});
