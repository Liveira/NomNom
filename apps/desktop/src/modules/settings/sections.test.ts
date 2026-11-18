import { describe, expect, it } from 'vitest';
import { SECTIONS, matchSections } from './sections';
import { MOTIONS, THEMES } from './themes';

describe('settings search', () => {
  it('shows every section when nothing is typed', () => {
    expect(matchSections('')).toHaveLength(SECTIONS.length);
    expect(matchSections('   ')).toHaveLength(SECTIONS.length);
  });

  it('finds a section by a word that is only in its keywords', () => {
    expect(matchSections('hotkey').map((s) => s.id)).toEqual(['shortcuts']);
    expect(matchSections('firefox').map((s) => s.id)).toContain('browser');
    expect(matchSections('synthwave').map((s) => s.id)).toEqual(['appearance']);
  });

  it('needs every typed word to match', () => {
    expect(matchSections('theme zzz')).toEqual([]);
    expect(matchSections('theme motion').map((s) => s.id)).toEqual(['appearance']);
  });

  it('ignores case', () => {
    expect(matchSections('UPDATE').map((s) => s.id)).toContain('updates');
  });
});

describe('theme list', () => {
  it('has system plus seven palettes, each once', () => {
    const ids = THEMES.map((t) => t.id);
    expect(new Set(ids).size).toBe(ids.length);
    expect(ids).toEqual(['system', 'dark', 'light', 'aurora', 'synthwave', 'sakura', 'ocean', 'forest']);
  });
  it('offers three motion levels', () => {
    expect(MOTIONS.map((m) => m.id)).toEqual(['full', 'calm', 'off']);
  });
});
