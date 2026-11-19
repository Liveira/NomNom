import { describe, expect, it } from 'vitest';
import type { StartupItem } from '../../lib/ipc';
import { delayText, impactOf, sortItems } from './logic';

const item = (name: string, enabled: boolean, avgMs: number | null): StartupItem => ({
  id: name,
  name,
  command: name,
  program: name,
  enabled,
  can_change: true,
  from: '',
  icon_url: null,
  effect: avgMs === null ? null : { times: 2, total_ms: avgMs * 2, worst_ms: avgMs, last_ms: avgMs, last_when: '' },
});

describe('startup helpers', () => {
  it('writes delays the way people read them', () => {
    expect(delayText(4500)).toBe('4.5 s');
    expect(delayText(25_821)).toBe('26 s');
    expect(delayText(72_345)).toBe('1 min 12 s');
  });
  it('calls a program heavy, light or harmless by its average delay', () => {
    expect(impactOf(null)).toBe('none');
    expect(impactOf(item('a', true, 20_000).effect)).toBe('high');
    expect(impactOf(item('a', true, 7_000).effect)).toBe('medium');
    expect(impactOf(item('a', true, 1_000).effect)).toBe('low');
  });
  it('lists the turned-on ones first, the heaviest among them first', () => {
    const sorted = sortItems([item('quiet', true, null), item('off-heavy', false, 30_000), item('heavy', true, 30_000), item('light', true, 1_000)]);
    expect(sorted.map((i) => i.name)).toEqual(['heavy', 'light', 'quiet', 'off-heavy']);
  });
});
