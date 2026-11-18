/** What the Disk Analyzer shows and says. Pure, so it can be tested. */
import type { StDrive, StNode, StProgress, StSort } from '../../lib/ipc';
import { formatBytes } from '../../lib/format';

export type DisplayUnit = 'auto' | 'GB' | 'MB' | 'KB' | 'B';

export function formatUnitBytes(bytes: number, unit: DisplayUnit = 'auto'): string {
  if (unit === 'auto') return formatBytes(bytes);
  if (unit === 'B') return `${Math.round(bytes).toLocaleString()} B`;
  const div = unit === 'KB' ? 1024 : unit === 'MB' ? 1024 ** 2 : 1024 ** 3;
  const val = bytes / div;
  if (val === 0) return `0 ${unit}`;
  if (val >= 100) return `${val.toFixed(0)} ${unit}`;
  if (val >= 1) return `${val.toFixed(1)} ${unit}`;
  return `${val.toFixed(2)} ${unit}`;
}

export function usagePercent(used: number, total: number): number {
  return total > 0 ? Math.min(100, Math.max(0, (used / total) * 100)) : 0;
}

export function driveKindLabel(d: Pick<StDrive, 'kind' | 'media'>): string {
  switch (d.kind) {
    case 'fixed':
      return d.media === 'ssd' ? 'SSD' : d.media === 'hdd' ? 'Hard disk' : 'Internal drive';
    case 'removable':
      return 'Removable drive';
    case 'network':
      return 'Network drive';
    case 'optical':
      return 'Disc drive';
    case 'ram':
      return 'RAM disk';
    default:
      return 'Drive';
  }
}

export function driveTitle(d: StDrive): string {
  if (d.root.startsWith('/')) {
    if (d.root === '/') return d.label ? `Root (${d.label})` : 'Root filesystem (/)';
    return d.label ? `${d.root} (${d.label})` : d.root;
  }
  return d.label ? `${d.label} (${d.letter}:)` : `Local disk (${d.letter}:)`;
}

export function formatDate(unix: number | null | undefined): string {
  if (unix == null) return '';
  return new Date(unix * 1000).toLocaleDateString([], { year: 'numeric', month: 'short', day: 'numeric' });
}

export function formatElapsed(ms: number): string {
  const s = Math.floor(ms / 1000);
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  const two = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${two(m % 60)}:${two(s % 60)}` : `${m}:${two(s % 60)}`;
}

export function stateLabel(p: Pick<StProgress, 'state'>): string {
  switch (p.state) {
    case 'running':
      return 'Scanning…';
    case 'paused':
      return 'Paused';
    case 'done':
      return 'Scan complete';
    default:
      return 'Scan stopped. The result is partial.';
  }
}

export function isBusy(p: Pick<StProgress, 'state'> | null): boolean {
  return p != null && (p.state === 'running' || p.state === 'paused');
}

export interface DriveSpaceAccount {
  freeBytes: number;
  systemBytes: number;
  mftBytes: number;
  totalBytes: number;
}

export function driveAccounting(p: StProgress | null, isWholeDrive: boolean): DriveSpaceAccount | null {
  if (!p || !isWholeDrive || p.drive_total == null || p.drive_used == null) return null;
  const freeBytes = Math.max(0, p.drive_free ?? p.drive_total - p.drive_used);
  const systemBytes = Math.max(0, p.drive_used - p.allocated);
  return {
    freeBytes,
    systemBytes,
    mftBytes: p.mft_bytes ?? 0,
    totalBytes: p.drive_total,
  };
}

/**
 * Compares what was found with what Windows says is used. Only says something once the scan is
 * over, and only reports the real numbers: a gap is explained, never hidden or padded.
 */
export function completenessNote(p: StProgress, isWholeDrive: boolean): string | null {
  if (!isWholeDrive || p.drive_used == null || p.state === 'running' || p.state === 'paused') return null;
  const found = p.allocated;
  const used = p.drive_used;
  const gap = used - found;
  const mft = p.mft_bytes ? ` NTFS file table: ${formatBytes(p.mft_bytes)}.` : '';
  const head = `Found ${formatBytes(p.bytes)} in files, ${formatBytes(found)} on disk.${mft} Windows reports ${formatBytes(used)} used on this drive.`;
  if (p.state === 'cancelled') return `${head} The scan was stopped, so the result is partial.`;
  if (Math.abs(gap) < Math.max(used * 0.01, 50 * 1024 * 1024)) return head;
  const why =
    gap > 0
      ? 'The difference can be system files and reserved space (page file, hibernation file, restore points, file system metadata), or folders NomNom could not open.'
      : 'The difference can be files with more than one name, or space compressed or shared by Windows.';
  return `${head} ${why}${p.errors > 0 ? ` NomNom couldn't open ${p.errors} ${p.errors === 1 ? 'place' : 'places'}.` : ''}`;
}

export interface Column {
  key: string;
  label: string;
  width: number;
  align: 'left' | 'right';
  sort: StSort | null;
  defaultHidden?: boolean;
}

export const COLUMNS: Column[] = [
  { key: 'name', label: 'Name', width: 320, align: 'left', sort: 'name' },
  { key: 'size', label: 'Size', width: 100, align: 'right', sort: 'size' },
  { key: 'alloc', label: 'On disk', width: 100, align: 'right', sort: 'alloc' },
  { key: 'percent', label: '% of folder', width: 130, align: 'left', sort: 'percent' },
  { key: 'own_size', label: 'Own size', width: 100, align: 'right', sort: 'own_size', defaultHidden: true },
  { key: 'wasted', label: 'Slack / saved', width: 110, align: 'right', sort: 'wasted', defaultHidden: true },
  { key: 'files', label: 'Files', width: 80, align: 'right', sort: 'files' },
  { key: 'dirs', label: 'Folders', width: 80, align: 'right', sort: 'dirs' },
  { key: 'owner', label: 'Owner', width: 130, align: 'left', sort: 'owner', defaultHidden: true },
  { key: 'modified', label: 'Modified', width: 120, align: 'left', sort: 'modified' },
  { key: 'created', label: 'Created', width: 120, align: 'left', sort: 'created', defaultHidden: true },
  { key: 'accessed', label: 'Last opened', width: 120, align: 'left', sort: 'accessed', defaultHidden: true },
  { key: 'type', label: 'Type', width: 90, align: 'left', sort: 'type', defaultHidden: true },
  { key: 'attributes', label: 'Attributes', width: 90, align: 'left', sort: null, defaultHidden: true },
  { key: 'path_len', label: 'Path length', width: 95, align: 'right', sort: 'path_len', defaultHidden: true },
];

export interface ColumnState {
  widths: Record<string, number>;
  hidden: string[];
  order: string[];
}

export const MIN_COLUMN_WIDTH = 50;

export function defaultColumnState(): ColumnState {
  return {
    widths: {},
    hidden: COLUMNS.filter((c) => c.defaultHidden).map((c) => c.key),
    order: COLUMNS.map((c) => c.key),
  };
}

/** Columns in the user's order, minus hidden ones, with their widths. "Name" is always shown. */
export function visibleColumns(state: ColumnState): (Column & { w: number })[] {
  const byKey = new Map(COLUMNS.map((c) => [c.key, c]));
  const order = [...state.order.filter((k) => byKey.has(k)), ...COLUMNS.map((c) => c.key).filter((k) => !state.order.includes(k))];
  return order
    .map((k) => byKey.get(k)!)
    .filter((c) => c.key === 'name' || !state.hidden.includes(c.key))
    .map((c) => ({ ...c, w: Math.max(MIN_COLUMN_WIDTH, state.widths[c.key] ?? c.width) }));
}

export function moveColumn(order: string[], key: string, before: string): string[] {
  if (key === before || !order.includes(key)) return order;
  const without = order.filter((k) => k !== key);
  const at = without.indexOf(before);
  if (at < 0) return order;
  without.splice(at, 0, key);
  return without;
}

export function parseColumnState(text: string | null): ColumnState {
  const base = defaultColumnState();
  if (!text) return base;
  try {
    const v = JSON.parse(text);
    const knownOrder = Array.isArray(v.order) ? v.order.filter((k: unknown) => typeof k === 'string') : null;
    const savedHidden = Array.isArray(v.hidden) ? v.hidden.filter((k: unknown) => typeof k === 'string') : null;
    // Newly added optional columns that weren't in an older saved `order` stay hidden until ticked.
    const extraHidden =
      knownOrder && savedHidden
        ? COLUMNS.filter((c) => c.defaultHidden && !knownOrder.includes(c.key) && !savedHidden.includes(c.key)).map((c) => c.key)
        : [];
    return {
      widths: typeof v.widths === 'object' && v.widths ? v.widths : {},
      hidden: savedHidden ? [...savedHidden, ...extraHidden] : base.hidden,
      order: knownOrder ?? base.order,
    };
  } catch {
    return base;
  }
}

export function compactAttributes(attrs: number, flags: number): string {
  let out = '';
  if (attrs & 0x1) out += 'R';
  if (attrs & 0x2) out += 'H';
  if (attrs & 0x4) out += 'S';
  if (attrs & 0x20) out += 'A';
  if (attrs & 0x800 || flags & 64) out += 'C';
  if (attrs & 0x4000) out += 'E';
  if (attrs & 0x200 || flags & 32) out += 'P';
  if (flags & 2) out += 'L';
  if (flags & 128) out += '+';
  return out;
}

export function itemTypeLabel(n: Pick<StNode, 'name' | 'is_dir' | 'is_link'>): string {
  if (n.is_link) return 'Link';
  if (n.is_dir) return 'Folder';
  const dot = n.name.lastIndexOf('.');
  return dot > 0 && dot + 1 < n.name.length ? `.${n.name.slice(dot + 1).toLowerCase()}` : 'File';
}

export function formatSlack(size: number, alloc: number, unit: DisplayUnit = 'auto'): { text: string; kind: 'saved' | 'slack' | 'even' } {
  const diff = alloc - size;
  if (Math.abs(diff) < 64) return { text: '0 B', kind: 'even' };
  if (diff < 0) return { text: `−${formatUnitBytes(-diff, unit)}`, kind: 'saved' };
  return { text: `+${formatUnitBytes(diff, unit)}`, kind: 'slack' };
}

export function cellText(colKey: string, n: StNode, unit: DisplayUnit = 'auto'): string {
  switch (colKey) {
    case 'size':
      return formatUnitBytes(n.size, unit);
    case 'alloc':
      return n.is_dir || n.alloc > 0 ? formatUnitBytes(n.alloc, unit) : '0 B';
    case 'own_size':
      return formatUnitBytes(n.is_dir ? (n.own_size ?? 0) : n.size, unit);
    case 'wasted':
      return formatSlack(n.size, n.alloc, unit).text;
    case 'files':
      return n.is_dir ? n.files.toLocaleString() : '';
    case 'dirs':
      return n.is_dir ? n.dirs.toLocaleString() : '';
    case 'owner':
      return n.owner ?? '';
    case 'modified':
      return formatDate(n.mtime);
    case 'created':
      return formatDate(n.ctime);
    case 'accessed':
      return formatDate(n.atime);
    case 'type':
      return itemTypeLabel(n);
    case 'attributes':
      return compactAttributes(n.attrs, n.flags);
    case 'path_len':
      return n.path_len != null ? String(n.path_len) : '';
    default:
      return '';
  }
}

/**
 * Selection rules of a file list: a plain click selects one, Ctrl toggles one, Shift selects the
 * range from the anchor. `ids` is the list in display order.
 */
export function nextSelection(
  selected: Set<number>,
  anchor: number | null,
  ids: number[],
  index: number,
  mods: { ctrl: boolean; shift: boolean },
): { selected: Set<number>; anchor: number | null } {
  const id = ids[index];
  if (id === undefined) return { selected, anchor };
  if (mods.shift && anchor != null) {
    const a = ids.indexOf(anchor);
    if (a >= 0) {
      const [lo, hi] = a < index ? [a, index] : [index, a];
      const range = ids.slice(lo, hi + 1);
      return { selected: new Set(mods.ctrl ? [...selected, ...range] : range), anchor };
    }
  }
  if (mods.ctrl) {
    const next = new Set(selected);
    if (next.has(id)) next.delete(id);
    else next.add(id);
    return { selected: next, anchor: id };
  }
  return { selected: new Set([id]), anchor: id };
}

/** Rankings that mention folders must say that parent and child overlap. */
export const OVERLAP_NOTE =
  'A folder includes everything inside it, so a folder and its subfolders overlap and these sizes must not be added together. Choose "Files directly inside" to see sizes that do add up.';

export const SIZE_UNITS: Record<string, number> = { KB: 1024, MB: 1024 ** 2, GB: 1024 ** 3 };

/** "500", "1.5" and a unit to bytes. Empty means no limit; anything else that isn't a number is an error. */
export function parseSize(text: string, unit: string): number | null | 'bad' {
  const t = text.trim().replace(',', '.');
  if (t === '') return null;
  if (!/^\d+(\.\d+)?$/.test(t)) return 'bad';
  const mult = SIZE_UNITS[unit] ?? 1;
  return Math.round(Number(t) * mult);
}

export type Action = 'recycle' | 'delete' | 'copy' | 'cut' | 'paste' | 'rename' | 'refresh' | 'properties' | 'selectAll';

/** The keyboard shortcuts of the file list. Returns what a key press means, or null. */
export function shortcutAction(k: { key: string; ctrl: boolean; shift: boolean; alt: boolean }): Action | null {
  const key = k.key.length === 1 ? k.key.toLowerCase() : k.key;
  if (key === 'Delete') return k.shift && !k.ctrl && !k.alt ? 'delete' : !k.shift && !k.ctrl && !k.alt ? 'recycle' : null;
  if (key === 'F2' && !k.ctrl && !k.shift && !k.alt) return 'rename';
  if (key === 'F5' && !k.ctrl && !k.shift && !k.alt) return 'refresh';
  if (key === 'Enter' && k.alt && !k.ctrl && !k.shift) return 'properties';
  if (k.ctrl && !k.alt && !k.shift) {
    if (key === 'c') return 'copy';
    if (key === 'x') return 'cut';
    if (key === 'v') return 'paste';
    if (key === 'a') return 'selectAll';
  }
  return null;
}

/** Shortcuts only work when the person is looking at the file list and isn't typing or in a dialog. */
export function shortcutsActive(ctx: { dialogOpen: boolean; menuOpen: boolean; targetIsField: boolean; tab: string; hasSession: boolean; busy: boolean }): boolean {
  return ctx.hasSession && ctx.tab === 'explorer' && !ctx.dialogOpen && !ctx.menuOpen && !ctx.targetIsField && !ctx.busy;
}

export function attributeLabels(attrs: number, flags: number): string[] {
  const out: string[] = [];
  if (attrs & 0x1) out.push('Read-only');
  if (attrs & 0x2) out.push('Hidden');
  if (attrs & 0x4) out.push('System');
  if (attrs & 0x20) out.push('Ready to archive');
  if (attrs & 0x800 || flags & 64) out.push('Compressed');
  if (attrs & 0x4000) out.push('Encrypted');
  if (attrs & 0x1000) out.push('Cloud placeholder');
  if (attrs & 0x200 || flags & 32) out.push('Sparse');
  if (flags & 2) out.push('Link (not followed)');
  if (flags & 4) out.push('Same data as another name');
  if (flags & 16) out.push('Partly unreadable');
  if (flags & 128) out.push('Has alternate streams');
  return out;
}
