import { describe, expect, it } from 'vitest';
import type { IntegrationStatus } from '../../lib/ipc';
import { autostartLine, menuLine, needsRepair } from './integration';

const status = (shell: Partial<IntegrationStatus['shell']>): IntegrationStatus => ({
  shell: { enabled: true, mode: 'none', note: null, ...shell },
  autostart: false,
  has_modern_package: false,
});

describe('menuLine', () => {
  it('says plainly when it is off', () => {
    expect(menuLine(status({ enabled: false }))).toContain('Off');
  });
  it('names the Windows 11 menu when it is in use', () => {
    expect(menuLine(status({ mode: 'modern' }))).toBe('In the Windows 11 right-click menu.');
  });
  it('explains the classic menu and why the new one is not used', () => {
    expect(menuLine(status({ mode: 'classic' }))).toContain('Show more options');
    const line = menuLine(
      status({ mode: 'classic', note: "Windows doesn't trust the menu package's signature yet. Using the classic menu (Show more options)." }),
    );
    expect(line).toContain("doesn't trust");
    expect(line.match(/Show more options/g)).toHaveLength(1);
  });
  it('points at Repair when it is on but missing', () => {
    expect(menuLine(status({ mode: 'none' }))).toContain('Repair');
  });
});

describe('needsRepair', () => {
  it('only when it should be there and is not', () => {
    expect(needsRepair(status({ mode: 'none' }))).toBe(true);
    expect(needsRepair(status({ mode: 'classic' }))).toBe(false);
    expect(needsRepair(status({ enabled: false }))).toBe(false);
  });
});

describe('autostartLine', () => {
  it('describes both states', () => {
    expect(autostartLine(true)).toContain('tray');
    expect(autostartLine(false)).toContain('only when you open it');
  });
});
