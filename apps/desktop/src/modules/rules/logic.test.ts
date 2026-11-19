import { describe, expect, it } from 'vitest';
import { blankRule, describeRule, describeStep, leaf, newStep, parentOf, presets } from './logic';

describe('folder rules helpers', () => {
  it('takes the last part of a path from either kind of separator', () => {
    expect(leaf('C:\\Users\\a\\Pictures')).toBe('Pictures');
    expect(leaf('/home/a/Pictures/')).toBe('Pictures');
  });
  it('finds the folder a path is in', () => {
    expect(parentOf('C:\\Users\\a\\out\\cat.webp')).toBe('C:\\Users\\a\\out');
    expect(parentOf('/home/a/out/cat.webp')).toBe('/home/a/out');
  });
  it('starts a new rule with one step and nothing deleted', () => {
    const r = blankRule('C:\\Users\\a\\Downloads');
    expect(r.enabled).toBe(true);
    expect(r.steps).toHaveLength(1);
    expect(r.steps.every((s) => ['move', 'copy', 'rename', 'convert', 'run', 'notify'].includes(s.type))).toBe(true);
  });
  it('describes steps and rules in plain words', () => {
    expect(describeStep({ type: 'move', folder: 'C:\\x\\Docs' })).toBe('move to Docs');
    expect(describeStep({ type: 'convert', to: 'webp', originals: '' })).toBe('make it WEBP');
    const r = { ...blankRule('C:\\Users\\a\\Downloads'), extensions: ['png', 'jpg'], min_kb: 100, steps: [newStep('convert'), { type: 'move' as const, folder: 'C:\\p' }] };
    expect(describeRule(r)).toBe('png, jpg (over 100 KB) in Downloads: make it WEBP, then move to p');
  });
  it('builds its presets from the downloads folder', () => {
    const list = presets('C:\\Users\\a\\Downloads');
    expect(list.length).toBeGreaterThan(1);
    const r = list[0].rule('', '\\');
    expect(r.folder).toBe('C:\\Users\\a\\Downloads');
    expect(JSON.stringify(r.steps)).toContain('C:\\\\Users\\\\a\\\\Pictures\\\\Converted');
    const linux = presets('/home/a/Downloads')[1].rule('', '/');
    expect(JSON.stringify(linux.steps)).toContain('/home/a/Documents/PDFs');
  });
});
