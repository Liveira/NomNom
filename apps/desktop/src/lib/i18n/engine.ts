/**
 * The translation engine: a catalog of [English, Português, Español] entries and a function that
 * turns an English interface string into the chosen language. English is always the key, so a
 * missing entry just shows the English text.
 *
 * An entry may hold `{}` for a value that changes ("Checked {} hours ago."). Values are carried
 * over in order, or by number ({2}) when the order differs between languages.
 */
export type Lang = 'en' | 'pt' | 'es';
export type Entry = [en: string, pt: string, es: string];

export interface Catalog {
  exact: Map<string, string>;
  patterns: { re: RegExp; out: string }[];
}

const SPACES = /\s+/g;
const collapse = (s: string): string => s.replace(SPACES, ' ').trim();
const escape = (s: string): string => s.replace(/[.*+?^$()|[\]\\]/g, '\\$&');

export function buildCatalog(entries: Entry[], lang: Lang): Catalog {
  const exact = new Map<string, string>();
  const patterns: { re: RegExp; out: string }[] = [];
  const col = lang === 'pt' ? 1 : lang === 'es' ? 2 : 0;
  for (const e of entries) {
    const key = collapse(e[0]);
    const out = e[col] ?? e[0];
    if (lang === 'en' || !out) continue;
    if (key.includes('{}')) {
      const re = new RegExp('^' + key.split('{}').map(escape).join('(.+?)') + '$', 's');
      patterns.push({ re, out });
    } else {
      exact.set(key, out);
    }
  }
  // Longer patterns first, so the most specific one wins.
  patterns.sort((a, b) => b.re.source.length - a.re.source.length);
  return { exact, patterns };
}

function fill(out: string, groups: string[]): string {
  let next = 0;
  return out.replace(/\{(\d*)\}/g, (_m, n: string) => groups[n ? Number(n) - 1 : next++] ?? '');
}

/** Translates one piece of text, keeping the spaces around it. */
export function translateText(cat: Catalog, text: string, memo?: Map<string, string>): string {
  const key = collapse(text);
  if (!key) return text;
  let hit = memo?.get(key);
  if (hit === undefined) {
    hit = cat.exact.get(key);
    if (hit === undefined) {
      for (const p of cat.patterns) {
        const m = p.re.exec(key);
        if (m) {
          // The values inside may be translatable too ("Today", "Unknown").
          hit = fill(p.out, m.slice(1).map((g) => cat.exact.get(g) ?? g));
          break;
        }
      }
    }
    if (hit === undefined) hit = '';
    if (memo) {
      if (memo.size > 6000) memo.clear();
      memo.set(key, hit);
    }
  }
  if (!hit) return text;
  const lead = /^\s*/.exec(text)![0];
  const tail = /\s*$/.exec(text)![0];
  return lead + hit + tail;
}

/** The language to use when the setting is "auto": the first of the system's languages we have. */
export function detectLang(languages: readonly string[]): Lang {
  for (const l of languages) {
    const code = l.toLowerCase().split('-')[0];
    if (code === 'pt') return 'pt';
    if (code === 'es') return 'es';
    if (code === 'en') return 'en';
  }
  return 'en';
}
