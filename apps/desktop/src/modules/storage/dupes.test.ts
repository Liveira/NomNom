import { describe, expect, it } from 'vitest';
import type { StDupGroup } from '../../lib/ipc';
import { defaultKeep, initialChoice, setKeep, summarize, toggleRemove, toRequest } from './dupes';

const group = (copies: [number, string, number | null, number?][], size = 1000): StDupGroup => ({
  size, sha256: 'x', recoverable: size * (copies.length - 1),
  copies: copies.map(([node, path, mtime, names]) => ({ node, path, mtime, names: names ?? 1 })),
});

describe('which copy stays', () => {
  const g = group([[1, 'C:\\b\\new.bin', 300], [2, 'C:\\a\\old.bin', 100], [3, 'C:\\a\\deeper\\old2.bin', 100]]);
  it('is the oldest, then the shortest path', () => {
    expect(defaultKeep(g)).toBe(2);
    expect(defaultKeep(group([[1, 'x', null], [2, 'y', 5]]))).toBe(2);
  });
  it('starts with every other copy marked, never the kept one', () => {
    const c = initialChoice(g);
    expect([...c.remove].sort()).toEqual([1, 3]);
    expect(c.remove.has(c.keep)).toBe(false);
  });
  it('lets the person move the keeper but never marks it for removal', () => {
    let c = initialChoice(g);
    c = setKeep(c, 1);
    expect(c.keep).toBe(1);
    expect(c.remove.has(1)).toBe(false);
    expect(toggleRemove(c, 1)).toBe(c);
    c = toggleRemove(c, 3);
    expect(c.remove.has(3)).toBe(false);
    c = toggleRemove(c, 3);
    expect(c.remove.has(3)).toBe(true);
  });
});

describe('summary and request', () => {
  const g1 = group([[1, 'a', 1], [2, 'b', 2], [3, 'c', 3]], 1000);
  const g2 = group([[4, 'd', 1], [5, 'e', 2, 2]], 500); // 5 has two names: removing one name frees nothing
  const groups = [g1, g2];
  const choices = new Map([[0, initialChoice(g1)], [1, initialChoice(g2)]]);
  it('counts files and only the space that really comes back', () => {
    expect(summarize(groups, choices)).toEqual({ files: 3, bytes: 2000, groups: 2 });
  });
  it('asks only for groups with something to remove, and always names the keeper', () => {
    const none = new Map([[0, { keep: 1, remove: new Set<number>() }]]);
    expect(toRequest(groups, none)).toEqual([]);
    expect(toRequest(groups, choices)).toEqual([{ keep: 1, remove: [2, 3] }, { keep: 4, remove: [5] }]);
  });
  it('never sends the keeper in the removal list, even if the state were damaged', () => {
    const bad = new Map([[0, { keep: 1, remove: new Set([1, 2]) }]]);
    expect(toRequest(groups, bad)).toEqual([{ keep: 1, remove: [2] }]);
  });
});
