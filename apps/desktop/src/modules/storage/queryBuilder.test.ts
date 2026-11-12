import { describe, expect, it } from 'vitest';
import { buildQuery, newRow, parseSaved, type Row } from './queryBuilder';

const row = (patch: Partial<Row>): Row => ({ ...newRow(patch.type ?? 'name'), ...patch });

describe('buildQuery', () => {
  it('builds the example from the request: mp4, over 1 GB, not changed for six months', () => {
    const r = buildQuery(
      [
        row({ type: 'ext', text: '.MP4' }),
        row({ type: 'size', min: '1', unit: 'GB' }),
        row({ type: 'age', field: 'modified', days: '180', dir: 'older' }),
      ],
      'and',
    );
    expect(r).toEqual({
      query: {
        type: 'and',
        items: [
          { type: 'ext', values: ['mp4'] },
          { type: 'size', min: 1024 ** 3, max: null },
          { type: 'age', field: 'modified', older_than_days: 180, newer_than_days: null },
        ],
      },
    });
  });
  it('a single condition is not wrapped, and NOT wraps one condition', () => {
    expect(buildQuery([row({ text: 'report' })], 'or')).toEqual({ query: { type: 'name', text: 'report', mode: 'contains' } });
    expect(buildQuery([row({ type: 'ext', text: 'tmp', not: true }), row({ text: 'x' })], 'or')).toEqual({
      query: { type: 'or', items: [{ type: 'not', item: { type: 'ext', values: ['tmp'] } }, { type: 'name', text: 'x', mode: 'contains' }] },
    });
  });
  it('explains what is missing', () => {
    expect(buildQuery([], 'and')).toEqual({ error: 'Add at least one condition.' });
    expect('error' in buildQuery([row({ text: '  ' })], 'and')).toBe(true);
    expect('error' in buildQuery([row({ type: 'size' })], 'and')).toBe(true);
    expect(buildQuery([row({ type: 'size', min: '5', max: '1' })], 'and')).toEqual({ error: 'The smallest size is bigger than the biggest.' });
    expect(buildQuery([row({ type: 'age', days: 'many' })], 'and')).toEqual({ error: 'Give the number of days as a whole number.' });
    expect(buildQuery([row({ type: 'date', after: '31/03/2024' })], 'and')).toEqual({ error: 'Dates look like 2024-03-31.' });
  });
  it('reads dates as whole days, UTC', () => {
    const r = buildQuery([row({ type: 'date', after: '2024-01-01', before: '2024-01-01' })], 'and');
    expect(r).toEqual({ query: { type: 'date', field: 'modified', after: 1704067200, before: 1704067200 + 86399 } });
  });
});

describe('saved searches', () => {
  it('round-trip and survive damage', () => {
    const saved = [{ name: 'Old videos', join: 'and', rows: [row({ type: 'ext', text: 'mp4' })] }];
    const back = parseSaved(JSON.stringify(saved));
    expect(back).toHaveLength(1);
    expect(back[0].rows[0].text).toBe('mp4');
    expect(parseSaved('nope')).toEqual([]);
    expect(parseSaved(JSON.stringify([{ name: 5 }]))).toEqual([]);
    expect(parseSaved(null)).toEqual([]);
  });
});
