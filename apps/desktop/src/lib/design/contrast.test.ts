/// <reference types="node" />
/** Text on its background must be readable in both themes (WCAG AA: 4.5 for normal text). */
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { describe, expect, it } from 'vitest';

const css = readFileSync(resolve(process.cwd(), 'src/lib/design/tokens.css'), 'utf8');

function block(selector: RegExp): Record<string, string> {
  const m = css.match(selector);
  if (!m) throw new Error(`no block for ${selector}`);
  const out: Record<string, string> = {};
  for (const d of m[1].matchAll(/--([\w-]+):\s*([^;]+);/g)) out[d[1]] = d[2].trim();
  return out;
}

const dark = block(/:root\s*\{([^}]*)\}/);
const light = { ...dark, ...block(/:root\[data-theme='light'\]\s*\{([^}]*)\}/) };

function rgb(hex: string): [number, number, number] {
  const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
  if (!m) throw new Error(`not a plain hex colour: ${hex}`);
  return [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)];
}
function luminance([r, g, b]: [number, number, number]): number {
  const f = (c: number) => {
    const s = c / 255;
    return s <= 0.03928 ? s / 12.92 : ((s + 0.055) / 1.055) ** 2.4;
  };
  return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b);
}
export function ratio(a: string, b: string): number {
  const [l1, l2] = [luminance(rgb(a)), luminance(rgb(b))].sort((x, y) => y - x);
  return (l1 + 0.05) / (l2 + 0.05);
}

// [text colour, background it is drawn on]
const PAIRS: [string, string][] = [
  ['text', 'bg'], ['text', 'surface-1'], ['text', 'surface-2'], ['text', 'surface-3'],
  ['text-2', 'bg'], ['text-2', 'surface-1'], ['text-2', 'surface-2'],
  ['text-3', 'bg'], ['text-3', 'surface-1'], ['text-3', 'surface-2'],
  ['accent-text', 'bg'], ['accent-text', 'surface-1'],
  ['on-accent', 'accent'], ['on-accent', 'accent-hover'],
  ['danger', 'bg'], ['danger', 'surface-1'],
  ['ok', 'bg'], ['ok', 'surface-1'],
  ['info', 'bg'], ['info', 'surface-1'],
];

describe.each([
  ['dark', dark],
  ['light', light],
])('%s theme', (_name, theme) => {
  it.each(PAIRS)('%s on %s is readable', (fg, bg) => {
    const r = ratio(theme[fg], theme[bg]);
    expect(r, `${fg} ${theme[fg]} on ${bg} ${theme[bg]} is ${r.toFixed(2)}:1`).toBeGreaterThanOrEqual(4.5);
  });
});

describe('the focus ring', () => {
  it('stands out from the page in both themes (3:1 for interface parts)', () => {
    for (const [name, theme] of [['dark', dark], ['light', light]] as const) {
      for (const bg of ['bg', 'surface-1', 'surface-2']) {
        expect(ratio(theme.focus, theme[bg]), `${name}: focus on ${bg}`).toBeGreaterThanOrEqual(3);
      }
    }
  });
});
