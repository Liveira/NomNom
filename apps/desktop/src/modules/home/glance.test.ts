import { describe, expect, it } from 'vitest';
import { systemDisk } from './glance';

const d = (mount: string, total: number, removable = false) => ({ mount, total, removable });

describe('systemDisk', () => {
  it('prefers C: or the root', () => {
    expect(systemDisk([d('D:\\', 900), d('C:\\', 500)])?.mount).toBe('C:\\');
    expect(systemDisk([d('/home', 900), d('/', 500)])?.mount).toBe('/');
  });
  it('else takes the biggest fixed drive and skips removable and empty ones', () => {
    expect(systemDisk([d('E:\\', 100), d('F:\\', 800), d('G:\\', 2000, true), d('H:\\', 0)])?.mount).toBe('F:\\');
  });
  it('gives nothing when there is no fixed drive', () => {
    expect(systemDisk([d('G:\\', 2000, true)])).toBeNull();
    expect(systemDisk([])).toBeNull();
  });
});
