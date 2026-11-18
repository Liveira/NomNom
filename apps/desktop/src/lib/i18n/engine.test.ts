import { describe, expect, it } from 'vitest';
import { buildCatalog, detectLang, translateText, type Entry } from './engine';

const entries: Entry[] = [
  ['Settings', 'Configurações', 'Ajustes'],
  ['Checked {} hours ago.', 'Verificado há {} horas.', 'Comprobado hace {} horas.'],
  ['Showing the first {} of {}.', 'Mostrando os primeiros {} de {}.', 'Mostrando los primeros {} de {}.'],
  ['{} of {} done', '{2} de {1} concluídos', '{2} de {1} hechos'],
  ['Today', 'Hoje', 'Hoy'],
  ['Last seen {}', 'Visto por último {}', 'Visto por última vez {}'],
];

describe('translating', () => {
  const pt = buildCatalog(entries, 'pt');
  const es = buildCatalog(entries, 'es');

  it('translates exact text and keeps the spaces around it', () => {
    expect(translateText(pt, 'Settings')).toBe('Configurações');
    expect(translateText(es, '  Settings\n  ')).toBe('  Ajustes\n  ');
  });

  it('ignores line breaks and indentation inside the text', () => {
    expect(translateText(pt, 'Showing the first\n      100   of 2,000.')).toBe('Mostrando os primeiros 100 de 2,000.');
  });

  it('carries changing values over, in order or by number', () => {
    expect(translateText(pt, 'Checked 3 hours ago.')).toBe('Verificado há 3 horas.');
    expect(translateText(es, '4 of 10 done')).toBe('10 de 4 hechos');
  });

  it('translates a value that is itself a known word', () => {
    expect(translateText(pt, 'Last seen Today')).toBe('Visto por último Hoje');
  });

  it('leaves text it does not know exactly as it was', () => {
    expect(translateText(pt, 'My holiday photos.zip')).toBe('My holiday photos.zip');
    expect(translateText(buildCatalog(entries, 'en'), 'Settings')).toBe('Settings');
  });

  it('does not mistake a longer sentence for a short entry', () => {
    expect(translateText(pt, 'Settings and more')).toBe('Settings and more');
  });
});

describe('language detection', () => {
  it('uses the first language of the system that NomNom has', () => {
    expect(detectLang(['pt-BR', 'en-US'])).toBe('pt');
    expect(detectLang(['es-MX'])).toBe('es');
    expect(detectLang(['fr-FR', 'es-ES'])).toBe('es');
    expect(detectLang(['de'])).toBe('en');
    expect(detectLang([])).toBe('en');
  });
});
