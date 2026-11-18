import { open as pickDialog, save as saveFile } from '@tauri-apps/plugin-dialog';
import {
  stIpc,
  stTools,
  onStorageProgress,
  onStorageOp,
  type StDetail,
  type StDrive,
  type StNode,
  type StOpKind,
  type StOpProgress,
  type StPlan,
  type StProgress,
  type StReport,
  type StScanStartOpts,
  type StSort,
} from '../../lib/ipc';
import { isBusy, nextSelection, type DisplayUnit } from './logic';
import type { Row } from './queryBuilder';

export type Dialog =
  | { type: 'confirm'; plan: StPlan; busy: boolean; error: string }
  | { type: 'rename'; node: StNode; error: string }
  | { type: 'result'; report: StReport; kind: StOpKind }
  | { type: 'props'; node: StNode; path: string }
  | { type: 'refused'; plan: StPlan }
  | { type: 'batchrename'; nodes: number[] }
  | { type: 'zip'; nodes: number[]; error: string }
  | { type: 'export'; error: string };

const ROW_LIMIT = 5000;

class StorageStore {
  drives = $state<StDrive[]>([]);
  drivesLoading = $state(false);
  sessions = $state<StProgress[]>([]);
  sessionId = $state<number | null>(null);
  current = $state(0);
  detail = $state<StDetail | null>(null);
  rows = $state<StNode[]>([]);
  /** Known nodes currently visible in TreeGrid or FileTable so multi-level selections resolve. */
  knownNodes = new Map<number, StNode>();
  total = $state(0);
  sort = $state<StSort>('size');
  desc = $state(true);
  text = $state('');
  unit = $state<DisplayUnit>('auto');
  selected = $state(new Set<number>());
  anchor = $state<number | null>(null);
  /** Goes up whenever the scan's data changed, so views that cache reload. */
  version = $state(0);
  tab = $state<'explorer' | 'rankings' | 'charts' | 'search' | 'duplicates' | 'history'>('explorer');
  view = $state<'treegrid' | 'table' | 'treemap' | 'sunburst'>('treegrid');
  /** Set by the charts so the rankings or search open already filtered. */
  rankingCategory = $state('');
  rankingExtension = $state('');
  pendingSearch = $state<{ rows: Row[]; join: 'and' | 'or' } | null>(null);
  /** Scan preferences configured on the start screen. */
  scanCollectOwners = $state(true);
  scanCollectStreams = $state(false);
  scanExcludeText = $state('');

  clipboard = $state<{ session: number; nodes: number[]; cut: boolean } | null>(null);
  dialog = $state<Dialog | null>(null);
  menu = $state<{ x: number; y: number } | null>(null);
  opProgress = $state<StOpProgress | null>(null);
  notice = $state('');
  private noticeTimer: ReturnType<typeof setTimeout> | null = null;
  error = $state('');
  loading = $state(false);

  private lastReload = 0;
  private listening = false;
  private reloadTimer: ReturnType<typeof setTimeout> | null = null;

  get progress(): StProgress | null {
    return this.sessions.find((s) => s.id === this.sessionId) ?? null;
  }
  get busy(): boolean {
    return isBusy(this.progress);
  }
  /** True when the scan covers a whole drive (so its total can be compared with Windows' number). */
  get wholeDrive(): boolean {
    const p = this.progress;
    return !!p && /^[A-Za-z]:\\?$/.test(p.root);
  }

  registerNodes(list: StNode[]) {
    for (const n of list) {
      if (n.id >= 0) this.knownNodes.set(n.id, n);
    }
  }

  async init() {
    if (this.listening) return;
    this.listening = true;
    await onStorageProgress((p) => this.onProgress(p));
    await onStorageOp((p) => (this.opProgress = p));
    const existing = await stIpc.drives().catch(() => []);
    this.drives = existing;
    await this.checkInbox();
  }

  async checkInbox() {
    const folders = await stIpc.takeInbox().catch(() => []);
    for (const folder of folders) {
      await this.start(folder);
    }
  }

  async loadDrives() {
    this.drivesLoading = true;
    try {
      this.drives = await stIpc.drives();
    } catch (e) {
      this.error = String(e);
    } finally {
      this.drivesLoading = false;
    }
  }

  private onProgress(p: StProgress) {
    const i = this.sessions.findIndex((s) => s.id === p.id);
    if (i >= 0) this.sessions[i] = p;
    else this.sessions = [...this.sessions, p];
    if (p.id !== this.sessionId) return;
    const finished = p.state === 'done' || p.state === 'cancelled';
    const now = Date.now();
    if (finished || now - this.lastReload > 1000) {
      this.lastReload = now;
      this.version++;
      void this.reload();
    }
  }

  buildScanOpts(elevated = false): StScanStartOpts {
    const exclude = this.scanExcludeText
      .split(/[,;\n]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    return {
      elevated,
      collect_owners: this.scanCollectOwners,
      collect_streams: this.scanCollectStreams,
      exclude,
    };
  }

  async start(root: string, elevated = false) {
    this.error = '';
    try {
      const id = await stIpc.start(root, this.buildScanOpts(elevated));
      this.sessionId = id;
      this.current = 0;
      this.detail = null;
      this.rows = [];
      this.knownNodes.clear();
      this.total = 0;
      this.selected = new Set();
      this.tab = 'explorer';
      this.lastReload = 0;
      this.version++;
      if (!this.sessions.some((s) => s.id === id)) {
        this.sessions = [...this.sessions, await stIpc.progress(id)];
      }
      await this.reload();
    } catch (e) {
      this.error = String(e);
    }
  }

  async saveScanFile() {
    const id = this.sessionId;
    if (id == null) return;
    const path = await saveFile({
      title: 'Save scan file',
      defaultPath: 'scan.json',
      filters: [{ name: 'NomNom scan (.json)', extensions: ['json'] }],
    });
    if (typeof path !== 'string') return;
    try {
      const bytes = await stIpc.saveScan(id, path);
      this.say(`Scan file saved (${Math.max(1, Math.round(bytes / 1024)).toLocaleString()} KB).`);
    } catch (e) {
      this.error = String(e);
    }
  }

  async openScanFile() {
    const picked = await pickDialog({
      directory: false,
      multiple: false,
      title: 'Open saved scan',
      filters: [{ name: 'NomNom scan (.json)', extensions: ['json'] }],
    });
    if (typeof picked !== 'string') return;
    this.error = '';
    try {
      const id = await stIpc.loadScan(picked);
      const prog = await stIpc.progress(id);
      if (!this.sessions.some((s) => s.id === id)) {
        this.sessions = [...this.sessions, prog];
      }
      await this.select(id);
      this.say('Opened saved scan.');
    } catch (e) {
      this.error = String(e);
    }
  }

  async select(id: number) {
    this.sessionId = id;
    this.current = 0;
    this.selected = new Set();
    this.knownNodes.clear();
    this.version++;
    await this.reload();
  }

  async close(id: number) {
    await stIpc.close(id).catch(() => {});
    this.sessions = this.sessions.filter((s) => s.id !== id);
    if (this.sessionId === id) {
      this.sessionId = this.sessions[0]?.id ?? null;
      this.current = 0;
      this.rows = [];
      this.knownNodes.clear();
      this.detail = null;
      this.version++;
      if (this.sessionId != null) await this.reload();
    }
  }

  async pause() {
    if (this.sessionId != null) await stIpc.pause(this.sessionId).catch((e) => (this.error = String(e)));
  }
  async resume() {
    if (this.sessionId != null) await stIpc.resume(this.sessionId).catch((e) => (this.error = String(e)));
  }
  async cancel() {
    if (this.sessionId != null) await stIpc.cancel(this.sessionId).catch((e) => (this.error = String(e)));
  }

  /** Scans the folder being viewed again (or the whole thing when `whole`). */
  async refresh(whole = false) {
    const p = this.progress;
    if (!p || this.busy) return;
    if (whole || this.current === 0) {
      const tab = this.tab;
      const elevated = !!p.elevated;
      await this.close(p.id);
      await this.start(p.root, elevated);
      this.tab = tab;
      return;
    }
    try {
      await stIpc.rescan(p.id, this.current);
    } catch (e) {
      this.error = String(e);
    }
  }

  async go(node: number) {
    if (node < 0) return;
    this.current = node;
    this.selected = new Set();
    this.anchor = null;
    this.text = '';
    await this.reload();
  }

  async up() {
    const parent = this.detail?.node.parent;
    if (parent != null) await this.go(parent);
  }

  setSort(key: StSort) {
    if (this.sort === key) this.desc = !this.desc;
    else {
      this.sort = key;
      this.desc = key !== 'name' && key !== 'owner' && key !== 'type';
    }
    this.version++;
    void this.reload();
  }

  setText(t: string) {
    this.text = t;
    if (this.reloadTimer) clearTimeout(this.reloadTimer);
    this.reloadTimer = setTimeout(() => {
      this.version++;
      void this.reload();
    }, 200);
  }

  jumpToSearch(rows: Row[], join: 'and' | 'or' = 'and') {
    this.pendingSearch = { rows, join };
    this.tab = 'search';
  }

  async reload() {
    const id = this.sessionId;
    if (id == null) return;
    try {
      const [page, detail] = await Promise.all([
        stIpc.children(id, this.current, this.sort, this.desc, 0, ROW_LIMIT, this.text || undefined),
        stIpc.node(id, this.current),
      ]);
      if (id !== this.sessionId) return;
      this.rows = page.items;
      this.total = page.total;
      this.detail = detail;
      this.registerNodes(page.items);
      this.registerNodes(detail.trail);
    } catch (e) {
      this.error = String(e);
    }
  }

  clickRow(index: number, mods: { ctrl: boolean; shift: boolean }, idList?: number[]) {
    const ids = idList ?? this.rows.map((x) => x.id);
    const r = nextSelection(this.selected, this.anchor, ids, index, mods);
    // Virtual rows (id < 0) cannot be selected for file operations.
    this.selected = new Set([...r.selected].filter((id) => id >= 0));
    this.anchor = r.anchor != null && r.anchor >= 0 ? r.anchor : null;
  }

  selectAll(idList?: number[]) {
    const ids = (idList ?? this.rows.map((r) => r.id)).filter((id) => id >= 0);
    this.selected = new Set(ids);
  }

  // ---- file operations: plan, confirm, run ---------------------------------------------------

  say(text: string) {
    this.notice = text;
    if (this.noticeTimer) clearTimeout(this.noticeTimer);
    this.noticeTimer = setTimeout(() => (this.notice = ''), 7000);
  }

  get selectedRows(): StNode[] {
    const out: StNode[] = [];
    for (const id of this.selected) {
      const fromRows = this.rows.find((r) => r.id === id) ?? this.knownNodes.get(id);
      if (fromRows) out.push(fromRows);
    }
    return out;
  }

  async plan(kind: StOpKind, nodes: number[], dest?: string, newName?: string): Promise<StPlan | null> {
    const id = this.sessionId;
    const valid = nodes.filter((n) => n >= 0);
    if (id == null || valid.length === 0) return null;
    try {
      return await stIpc.opPlan(id, kind, valid, dest, newName);
    } catch (e) {
      this.error = String(e);
      return null;
    }
  }

  async runPlan(plan: StPlan, confirmed: boolean, typed?: string, includeReadonly = false): Promise<StReport | null> {
    this.opProgress = { done: 0, total: plan.item_count, current: '' };
    try {
      const report = await stIpc.opRun(plan.plan_id, confirmed, typed, includeReadonly);
      this.opProgress = null;
      await this.afterOp(report, plan.kind);
      return report;
    } catch (e) {
      this.opProgress = null;
      if (this.dialog?.type === 'confirm') this.dialog = { ...this.dialog, busy: false, error: String(e) };
      else this.error = String(e);
      return null;
    }
  }

  private async afterOp(report: StReport, kind: StOpKind) {
    this.dialog = null;
    this.selected = new Set();
    this.version++;
    await this.reload();
    const done = report.results.filter((r) => r.status === 'done').length;
    if (done < report.results.length || report.cancelled) {
      this.dialog = { type: 'result', report, kind };
    } else {
      const verb: Record<StOpKind, string> = {
        recycle: 'Moved to the Recycle Bin',
        delete: 'Deleted',
        copy: 'Copied',
        move: 'Moved',
        rename: 'Renamed',
        batch_rename: 'Renamed',
        zip: 'Added to a ZIP file',
        compress: 'Compressed on disk',
        decompress: 'Uncompressed on disk',
        hardlink: 'Linked identical copies',
      };
      this.say(`${verb[kind]}: ${done} ${done === 1 ? 'item' : 'items'}.`);
    }
  }

  show(plan: StPlan) {
    this.dialog = plan.item_count === 0 ? { type: 'refused', plan } : { type: 'confirm', plan, busy: false, error: '' };
  }

  async remove(permanent: boolean, nodes: number[] = [...this.selected]) {
    const plan = await this.plan(permanent ? 'delete' : 'recycle', nodes);
    if (!plan) return;
    if (!permanent && plan.item_count > 0 && plan.refused.length === 0 && plan.warnings.length === 0) {
      await this.runPlan(plan, false);
      return;
    }
    this.show(plan);
  }

  async compressSelection(compress: boolean, nodes: number[] = [...this.selected]) {
    const plan = await this.plan(compress ? 'compress' : 'decompress', nodes);
    if (plan) this.show(plan);
  }

  copySelection(cut: boolean) {
    const nodes = [...this.selected].filter((n) => n >= 0);
    if (nodes.length === 0 || this.sessionId == null) return;
    this.clipboard = { session: this.sessionId, nodes, cut };
    this.say(`${nodes.length} ${nodes.length === 1 ? 'item' : 'items'} ${cut ? 'cut' : 'copied'}. Open a folder and paste.`);
  }

  async paste() {
    const c = this.clipboard;
    const dest = this.detail?.path;
    if (!c || !dest || c.session !== this.sessionId) return;
    const plan = await this.plan(c.cut ? 'move' : 'copy', c.nodes, dest);
    if (plan) this.show(plan);
  }

  async sendTo(kind: 'copy' | 'move') {
    const nodes = [...this.selected].filter((n) => n >= 0);
    if (nodes.length === 0) return;
    const dest = await pickDialog({ directory: true, multiple: false, title: kind === 'copy' ? 'Copy to…' : 'Move to…' });
    if (typeof dest !== 'string') return;
    const plan = await this.plan(kind, nodes, dest);
    if (plan) this.show(plan);
  }

  startRename() {
    const only = this.selectedRows;
    if (only.length === 1) this.dialog = { type: 'rename', node: only[0], error: '' };
  }

  async doRename(newName: string) {
    if (this.dialog?.type !== 'rename') return;
    const node = this.dialog.node;
    const plan = await this.plan('rename', [node.id], undefined, newName);
    if (!plan) return;
    if (plan.item_count === 0) {
      this.dialog = { type: 'rename', node, error: plan.refused[0]?.reason ?? "That name can't be used." };
      return;
    }
    const report = await this.runPlan(plan, false);
    if (report && report.results[0]?.status !== 'done') {
      this.dialog = { type: 'rename', node, error: report.results[0]?.message ?? "It couldn't be renamed." };
    }
  }

  startBatchRename(nodes: number[] = [...this.selected]) {
    const valid = nodes.filter((n) => n >= 0);
    if (valid.length > 0) this.dialog = { type: 'batchrename', nodes: valid };
  }

  startZip(nodes: number[] = [...this.selected]) {
    const valid = nodes.filter((n) => n >= 0);
    if (valid.length > 0) this.dialog = { type: 'zip', nodes: valid, error: '' };
  }

  startExport() {
    if (this.sessionId != null) this.dialog = { type: 'export', error: '' };
  }

  async savePaths(nodes: number[] = [...this.selected]) {
    const id = this.sessionId;
    const valid = nodes.filter((n) => n >= 0);
    if (id == null || valid.length === 0) return;
    const path = await saveFile({ title: 'Save the list of paths', defaultPath: 'paths.txt', filters: [{ name: 'Text file', extensions: ['txt'] }] });
    if (typeof path !== 'string') return;
    try {
      const n = await stTools.exportPaths(id, valid, path);
      this.say(`Saved ${n} ${n === 1 ? 'path' : 'paths'}.`);
    } catch (e) {
      this.error = String(e);
    }
  }

  async showProperties() {
    const first = this.selectedRows[0]?.id ?? this.current;
    const id = this.sessionId;
    if (id == null || first < 0) return;
    const d = await stIpc.node(id, first);
    this.dialog = { type: 'props', node: d.node, path: d.path };
  }

  async openSelected(reveal: boolean) {
    const id = this.sessionId;
    const first = this.selectedRows[0];
    if (id == null || !first || first.id < 0) return;
    try {
      await stIpc.open(id, first.id, reveal);
    } catch (e) {
      this.error = String(e);
    }
  }

  async copyPath() {
    const id = this.sessionId;
    const valid = [...this.selected].filter((n) => n >= 0);
    if (id == null || valid.length === 0) return;
    const paths = await stIpc.paths(id, valid);
    await navigator.clipboard.writeText(paths.join('\r\n'));
    this.say(paths.length === 1 ? 'Path copied.' : `${paths.length} paths copied.`);
  }

  cancelOp() {
    void stIpc.opCancel();
  }

  get truncated(): boolean {
    return this.total > this.rows.length;
  }
}

export const storage = new StorageStore();
