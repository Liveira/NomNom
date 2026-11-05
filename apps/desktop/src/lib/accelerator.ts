/** Turns a keydown into an accelerator string like `Ctrl+Alt+D`, or null if it isn't one yet. */
export interface KeyLike {
  code: string;
  ctrlKey: boolean;
  altKey: boolean;
  shiftKey: boolean;
  metaKey: boolean;
}

const MODIFIER_CODES = /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/;

function keyName(code: string): string | null {
  if (MODIFIER_CODES.test(code)) return null;
  if (/^Key[A-Z]$/.test(code)) return code.slice(3);
  if (/^Digit[0-9]$/.test(code)) return code.slice(5);
  if (/^(F([1-9]|1[0-9]|2[0-4])|Space|Enter|Tab|Backspace|Delete|Insert|Home|End|PageUp|PageDown|Arrow(Up|Down|Left|Right))$/.test(code)) {
    return code;
  }
  return null;
}

export function toAccelerator(e: KeyLike): { accelerator: string | null; needsModifier: boolean } {
  const key = keyName(e.code);
  if (!key) return { accelerator: null, needsModifier: false };
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(Boolean);
  if (mods.length === 0) return { accelerator: null, needsModifier: true };
  return { accelerator: [...mods, key].join('+'), needsModifier: false };
}
