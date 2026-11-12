import { describe, expect, it } from 'vitest';
import type { StComparison } from '../../lib/ipc';
import { errorsNote, scheduleLabel, scopeNote, signedBytes, summaryLine, trendBars } from './history';

const cmp = (patch: Partial<StComparison> = {}): StComparison => ({
  bytes_before: 1024 * 1024 * 10, bytes_after: 1024 * 1024 * 12, delta: 1024 * 1024 * 2, files_before: 5, files_after: 6,
  depth: 6, big_min: 50 * 1024 * 1024, dirs: [], dirs_changed: 0, files: [], files_changed: 0, ...patch,
});

describe('how a difference is written', () => {
  it('has a sign and a size, or says nothing changed', () => {
    expect(signedBytes(1536)).toBe('+1.5 KB');
    expect(signedBytes(-300 * 1024)).toBe('-300 KB');
    expect(signedBytes(0)).toBe('no change');
  });
  it('the summary names both moments and the direction', () => {
    expect(summaryLine(cmp(), '1 Jan', 'now')).toBe('The total grew by 2.0 MB, from 10.0 MB (1 Jan) to 12.0 MB (now).');
    expect(summaryLine(cmp({ delta: -1024, bytes_after: 1024 * 1024 * 10 - 1024 }), 'a', 'b')).toContain('shrank by 1.0 KB');
    expect(summaryLine(cmp({ delta: 0 }), 'a', 'b')).toContain('did not change');
  });
  it('the scope note says what was left out', () => {
    expect(scopeNote(cmp())).toContain('down to 6 levels');
    expect(scopeNote(cmp())).toContain('50.0 MB');
    expect(scopeNote(cmp({ depth: 1 }))).toContain('1 level ');
  });
});

describe('the trend', () => {
  it('scales to the tallest bar and marks folders a snapshot did not keep', () => {
    const bars = trendBars([{ size: 50 }, { size: 100 }, { size: null }]);
    expect(bars.map((b) => b.height)).toEqual([0.5, 1, 0]);
    expect(bars.map((b) => b.missing)).toEqual([false, false, true]);
    expect(trendBars([{ size: 0 }])[0].height).toBe(0);
    expect(trendBars([])).toEqual([]);
  });
});

describe('small labels', () => {
  it('names the schedule and the unreadable places', () => {
    expect(scheduleLabel('daily')).toBe('Once a day');
    expect(scheduleLabel('off')).toBe('Off');
    expect(scheduleLabel('x')).toBe('Off');
    expect(errorsNote({ errors: 0 })).toBe('');
    expect(errorsNote({ errors: 1 })).toBe("1 place couldn't be read");
    expect(errorsNote({ errors: 3 })).toBe("3 places couldn't be read");
  });
});
