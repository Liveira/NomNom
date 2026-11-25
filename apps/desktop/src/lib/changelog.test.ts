import { describe, expect, it } from 'vitest';
import { CHANGE_KINDS, RELEASES, changelogStats, filterReleases, pickText } from './changelog';
import { CORE, TOOLS } from './tools';

describe('patch notes catalog', () => {
  it('covers every release in descending order with complete EN/PT/ES copy', () => {
    expect(RELEASES.length).toBeGreaterThanOrEqual(10);
    expect(RELEASES[0].latest).toBe(true);
    const seen = new Set<string>();
    const validIds = new Set([...CORE, ...TOOLS.map((t) => t.id)]);

    for (const r of RELEASES) {
      expect(r.version).toMatch(/^\d+\.\d+\.\d+$/);
      expect(seen.has(r.version), `duplicate version ${r.version}`).toBe(false);
      seen.add(r.version);
      expect(r.date).toMatch(/^\d{4}-\d{2}-\d{2}$/);
      expect(r.codename.en && r.codename.pt && r.codename.es).toBeTruthy();
      expect(r.summary.en && r.summary.pt && r.summary.es).toBeTruthy();
      expect(r.highlights.length).toBeGreaterThanOrEqual(1);
      for (const h of r.highlights) {
        expect(h.en && h.pt && h.es).toBeTruthy();
      }
      expect(r.changes.length).toBeGreaterThanOrEqual(1);
      for (const c of r.changes) {
        expect(CHANGE_KINDS).toContain(c.kind);
        expect(c.title.en && c.title.pt && c.title.es).toBeTruthy();
        expect(c.detail.en && c.detail.pt && c.detail.es).toBeTruthy();
        if (c.toolId) {
          expect(validIds.has(c.toolId), `unknown toolId ${c.toolId} in ${r.version}`).toBe(true);
        }
      }
    }
  });

  it('computes accurate aggregate statistics', () => {
    const stats = changelogStats(RELEASES);
    expect(stats.versions).toBe(RELEASES.length);
    expect(stats.totalChanges).toBe(stats.added + stats.improved + stats.design + stats.fixed);
    expect(stats.totalChanges).toBeGreaterThan(25);
  });

  it('filters releases by category and multilingual search queries', () => {
    const fixedOnly = filterReleases('', 'fixed');
    expect(fixedOnly.length).toBeGreaterThan(0);
    for (const r of fixedOnly) {
      for (const c of r.changes) expect(c.kind).toBe('fixed');
    }

    const webcamHits = filterReleases('webcam', 'all');
    expect(webcamHits.some((r) => r.version === '0.7.0')).toBe(true);

    const ptHits = filterReleases('arredondado', 'all');
    expect(ptHits.some((r) => r.version === '0.7.1')).toBe(true);

    const versionHits = filterReleases('v0.3.0', 'all');
    expect(versionHits.map((r) => r.version)).toEqual(['0.3.0']);

    expect(filterReleases('0.1.0', 'all').some((r) => r.version === '0.1.0')).toBe(true);

    expect(filterReleases('no-such-feature-xyz', 'all')).toEqual([]);
  });

  it('selects localized text cleanly for EN, PT and ES', () => {
    const sample = { en: 'Hello', pt: 'Olá', es: 'Hola' };
    expect(pickText(sample, 'en')).toBe('Hello');
    expect(pickText(sample, 'pt')).toBe('Olá');
    expect(pickText(sample, 'es')).toBe('Hola');
  });
});
