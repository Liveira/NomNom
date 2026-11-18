import { describe, expect, it } from 'vitest';
import { CATALOG } from './catalog';
import { buildCatalog, translateText } from './engine';

const key = (s: string) => s.replace(/\s+/g, ' ').trim();
const holes = (s: string) => (s.match(/\{\d*\}/g) ?? []).length;

describe('translation catalog', () => {
  it('has a Portuguese and a Spanish text for every English text', () => {
    for (const [en, pt, es] of CATALOG) {
      expect(en.trim(), 'empty English text').not.toBe('');
      expect(pt.trim(), `no Portuguese for "${en}"`).not.toBe('');
      expect(es.trim(), `no Spanish for "${en}"`).not.toBe('');
    }
  });

  it('keeps the same changing values in every language', () => {
    for (const [en, pt, es] of CATALOG) {
      expect(holes(pt), `Portuguese "${pt}" vs "${en}"`).toBe(holes(en));
      expect(holes(es), `Spanish "${es}" vs "${en}"`).toBe(holes(en));
    }
  });

  it('does not define the same English text twice with different translations', () => {
    const seen = new Map<string, [string, string]>();
    for (const [en, pt, es] of CATALOG) {
      const k = key(en);
      const old = seen.get(k);
      if (old) {
        expect(`${pt}|${es}`, `"${en}" is translated two ways`).toBe(`${old[0]}|${old[1]}`);
      }
      seen.set(k, [pt, es]);
    }
  });

  it('translates the menu in both languages', () => {
    const pt = buildCatalog(CATALOG, 'pt');
    const es = buildCatalog(CATALOG, 'es');
    expect(translateText(pt, 'Settings')).toBe('Configurações');
    expect(translateText(es, 'Settings')).toBe('Ajustes');
    expect(translateText(pt, 'Checked 5 minutes ago.')).toBe('Verificado há 5 minutos.');
    expect(translateText(es, 'Version 0.3.0 is available')).toBe('La versión 0.3.0 está disponible');
  });

  it('never touches text it does not know, such as a file name', () => {
    const pt = buildCatalog(CATALOG, 'pt');
    expect(translateText(pt, 'holiday-photos.zip')).toBe('holiday-photos.zip');
  });
});
