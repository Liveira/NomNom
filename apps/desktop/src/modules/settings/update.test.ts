import { describe, expect, it } from 'vitest';
import { installLine, lastCheckedLine, percent } from './update';

describe('update helpers', () => {
  it('describes when the last check happened', () => {
    expect(lastCheckedLine(0)).toBe('Not checked yet.');
    expect(lastCheckedLine(1000, 1010)).toBe('Checked just now.');
    expect(lastCheckedLine(1000, 1000 + 60)).toBe('Checked 1 minute ago.');
    expect(lastCheckedLine(1000, 1000 + 3 * 3600)).toBe('Checked 3 hours ago.');
    expect(lastCheckedLine(1000, 1000 + 5 * 86400)).toBe('Checked 5 days ago.');
  });

  it('computes download progress only when the size is known', () => {
    expect(percent(50, 200)).toBe(25);
    expect(percent(500, 200)).toBe(100);
    expect(percent(50, null)).toBeNull();
    expect(percent(50, 0)).toBeNull();
  });

  it('says what the install does', () => {
    const u = { version: '0.2.0', notes: '', page: '', file_name: 'a-setup.exe', url: '', size: 0, sha256: '' };
    expect(installLine(u)).toContain('checks it against the checksum');
    expect(installLine({ ...u, size: 5 * 1048576 })).toContain('5');
  });
});
