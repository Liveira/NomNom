/** Turns the rows of the search form into a query for the app. Pure, so it can be tested. */
import type { StDateField, StQuery, StTextMode } from '../../lib/ipc';
import { parseSize } from './logic';

export type RowType =
  | 'name' | 'path' | 'ext' | 'size' | 'age' | 'date' | 'attr' | 'kind' | 'item'
  | 'owner' | 'path_len' | 'empty_folder';

export interface Row {
  id: number;
  type: RowType;
  not: boolean;
  text: string;
  mode: StTextMode;
  min: string;
  max: string;
  unit: 'KB' | 'MB' | 'GB';
  field: StDateField;
  days: string;
  dir: 'older' | 'newer';
  /** A date typed as yyyy-mm-dd, for the "date" rows. */
  after: string;
  before: string;
  attr: string;
  category: string;
  what: 'file' | 'folder';
}

let counter = 0;
export function newRow(type: RowType = 'name'): Row {
  return {
    id: ++counter,
    type,
    not: false,
    text: '',
    mode: 'contains',
    min: type === 'path_len' ? '260' : '',
    max: '',
    unit: 'MB',
    field: 'modified',
    days: '',
    dir: 'older',
    after: '',
    before: '',
    attr: 'hidden',
    category: 'video',
    what: 'file',
  };
}

export const ROW_LABELS: Record<RowType, string> = {
  name: 'Name',
  path: 'Path',
  ext: 'Type (extension)',
  size: 'Size',
  age: 'Age',
  date: 'Date',
  owner: 'Owner (account)',
  path_len: 'Path length (chars)',
  empty_folder: 'Empty folder',
  attr: 'Attribute',
  kind: 'Kind of file',
  item: 'File or folder',
};

export interface SearchPreset {
  id: string;
  label: string;
  join: 'and' | 'or';
  make: () => Row[];
}

export const SEARCH_PRESETS: SearchPreset[] = [
  {
    id: 'large',
    label: 'Large files (> 100 MB)',
    join: 'and',
    make: () => [
      { ...newRow('item'), what: 'file' },
      { ...newRow('size'), min: '100', unit: 'MB' },
    ],
  },
  {
    id: 'old',
    label: 'Untouched (> 1 year)',
    join: 'and',
    make: () => [
      { ...newRow('item'), what: 'file' },
      { ...newRow('age'), field: 'modified', dir: 'older', days: '365' },
    ],
  },
  {
    id: 'temp',
    label: 'Temporary and logs',
    join: 'and',
    make: () => [{ ...newRow('kind'), category: 'temp' }],
  },
  {
    id: 'long_path',
    label: 'Long paths (260+ chars)',
    join: 'and',
    make: () => [{ ...newRow('path_len'), min: '260', max: '' }],
  },
  {
    id: 'empty_dirs',
    label: 'Empty folders',
    join: 'and',
    make: () => [newRow('empty_folder')],
  },
  {
    id: 'compressed',
    label: 'NTFS compressed',
    join: 'and',
    make: () => [{ ...newRow('attr'), attr: 'compressed' }],
  },
];

function day(text: string, endOfDay: boolean): number | null | 'bad' {
  const t = text.trim();
  if (!t) return null;
  const m = /^(\d{4})-(\d{2})-(\d{2})$/.exec(t);
  if (!m) return 'bad';
  const ms = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), endOfDay ? 23 : 0, endOfDay ? 59 : 0, endOfDay ? 59 : 0);
  return Number.isFinite(ms) ? Math.floor(ms / 1000) : 'bad';
}

function one(r: Row): StQuery | string {
  switch (r.type) {
    case 'name':
    case 'path':
    case 'owner':
      return r.text.trim() ? { type: r.type, text: r.text.trim(), mode: r.mode } : `Type some text for the ${r.type} condition.`;
    case 'path_len': {
      const parseLen = (s: string) => (s.trim() === '' ? null : /^\d+$/.test(s.trim()) ? Number(s.trim()) : 'bad');
      const min = parseLen(r.min);
      const max = parseLen(r.max);
      if (min === 'bad' || max === 'bad') return 'Path lengths must be whole numbers, like 260.';
      if (min == null && max == null) return 'Give a shortest or a longest path length.';
      if (min != null && max != null && min > max) return 'The shortest length is bigger than the longest.';
      return { type: 'path_len', min, max };
    }
    case 'empty_folder':
      return { type: 'empty_folder' };
    case 'ext': {
      const values = r.text.split(/[\s,;]+/).map((e) => e.replace(/^\./, '').toLowerCase()).filter(Boolean);
      return values.length ? { type: 'ext', values } : 'Type at least one file type, like mp4 or zip.';
    }
    case 'size': {
      const min = parseSize(r.min, r.unit);
      const max = parseSize(r.max, r.unit);
      if (min === 'bad' || max === 'bad') return 'Sizes must be plain numbers, like 500 or 1.5.';
      if (min == null && max == null) return 'Give a smallest or a biggest size.';
      if (min != null && max != null && min > max) return 'The smallest size is bigger than the biggest.';
      return { type: 'size', min, max };
    }
    case 'age': {
      if (!/^\d+$/.test(r.days.trim())) return 'Give the number of days as a whole number.';
      const n = Number(r.days.trim());
      return r.dir === 'older' ? { type: 'age', field: r.field, older_than_days: n, newer_than_days: null } : { type: 'age', field: r.field, older_than_days: null, newer_than_days: n };
    }
    case 'date': {
      const a = day(r.after, false);
      const b = day(r.before, true);
      if (a === 'bad' || b === 'bad') return 'Dates look like 2024-03-31.';
      if (a == null && b == null) return 'Give a start or an end date.';
      if (a != null && b != null && a > b) return 'The start date is after the end date.';
      return { type: 'date', field: r.field, after: a, before: b };
    }
    case 'attr':
      return { type: 'attr', name: r.attr, set: true };
    case 'kind':
      return { type: 'kind', category: r.category };
    case 'item':
      return { type: 'item', what: r.what };
  }
}

/** The whole query, or the first problem found, in plain words. */
export function buildQuery(rows: Row[], join: 'and' | 'or'): { query: StQuery } | { error: string } {
  if (rows.length === 0) return { error: 'Add at least one condition.' };
  const items: StQuery[] = [];
  for (const r of rows) {
    const q = one(r);
    if (typeof q === 'string') return { error: q };
    items.push(r.not ? { type: 'not', item: q } : q);
  }
  return { query: items.length === 1 ? items[0] : { type: join, items } };
}

export interface Saved {
  name: string;
  join: 'and' | 'or';
  rows: Row[];
}

export function parseSaved(text: string | null): Saved[] {
  if (!text) return [];
  try {
    const v = JSON.parse(text);
    if (!Array.isArray(v)) return [];
    return v
      .filter((s) => s && typeof s.name === 'string' && Array.isArray(s.rows) && (s.join === 'and' || s.join === 'or'))
      .map((s) => ({ name: s.name, join: s.join, rows: s.rows.map((r: Partial<Row>) => ({ ...newRow(r.type ?? 'name'), ...r, id: ++counter })) }));
  } catch {
    return [];
  }
}
