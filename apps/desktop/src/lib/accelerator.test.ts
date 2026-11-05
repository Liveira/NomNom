import { describe, expect, it } from 'vitest';
import { toAccelerator } from './accelerator';

const k = (code: string, mods: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}) =>
  toAccelerator({ code, ctrlKey: false, altKey: false, shiftKey: false, metaKey: false, ...mods });

describe('toAccelerator', () => {
  it('builds the default shortcut', () => {
    expect(k('KeyD', { ctrlKey: true, altKey: true }).accelerator).toBe('Ctrl+Alt+D');
  });
  it('names digits, function keys and arrows', () => {
    expect(k('Digit5', { ctrlKey: true, shiftKey: true }).accelerator).toBe('Ctrl+Shift+5');
    expect(k('F5', { altKey: true }).accelerator).toBe('Alt+F5');
    expect(k('ArrowUp', { ctrlKey: true }).accelerator).toBe('Ctrl+ArrowUp');
    expect(k('KeyW', { metaKey: true }).accelerator).toBe('Super+W');
  });
  it('waits while only modifiers are held', () => {
    expect(k('ControlLeft', { ctrlKey: true })).toEqual({ accelerator: null, needsModifier: false });
    expect(k('ShiftRight', { shiftKey: true }).accelerator).toBeNull();
  });
  it('asks for a modifier when a bare key is pressed', () => {
    expect(k('KeyD')).toEqual({ accelerator: null, needsModifier: true });
  });
  it('ignores keys it cannot name safely', () => {
    expect(k('Semicolon', { ctrlKey: true }).accelerator).toBeNull();
  });
});
