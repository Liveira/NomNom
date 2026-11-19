import { describe, expect, it } from 'vitest';
import { blankAction, blankParam, missingValues, placeholders, summary } from './logic';

describe('actions helpers', () => {
  it('finds each {value} once, in order', () => {
    expect(placeholders('a {x} b {y} {x} {} {1z}')).toEqual(['x', 'y']);
  });
  it('lists the values used but not defined yet', () => {
    const a = { ...blankAction(), target: 'https://x.org/?q={q}', args: '--lang {lang}', params: [blankParam('q')] };
    expect(missingValues(a)).toEqual(['lang']);
  });
  it('summarises a program with its arguments and a link as it is', () => {
    expect(summary({ kind: 'app', target: 'notepad', args: ' a.txt ' })).toBe('notepad a.txt');
    expect(summary({ kind: 'link', target: 'https://x.org', args: 'ignored' })).toBe('https://x.org');
  });
  it('starts as a program that shows in the palette, with no shortcut', () => {
    const a = blankAction();
    expect(a.kind).toBe('app');
    expect(a.in_palette).toBe(true);
    expect(a.shortcut).toBeNull();
  });
});
