import { open as pickFolder, save as saveFile } from '@tauri-apps/plugin-dialog';
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
  type StSort,
} from '../../lib/ipc';
import { isBusy, nextSelection } from './logic';

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
  total = $state(0);
  sort = $state<StSort>('size');
  desc = $state(true);
  text = $state('');
  selected = $state(new Set<number>());
  anchor = $state<number | null>(null);
  /** Goes up whenever the scan's data changed, so views that cache (the tree) reload. */
  version = $state(0);
  tab = $state<'explorer' | 'rankings' | 'charts' | 'search' | 'duplicates' | 'history'>('explorer');
  view = $state<'table' | 'treemap'>('table');
  /** Set by the charts so the rankings open already filtered; the rankings clear them. */
  rankingCategory = $state('');
  rankingExtension = $state('');
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

  async init() {
    if (this.listening) return;
    this.listening = true;
    await onStorageProgress((p) => this.onProgress(p));
    await onStorageOp((p) => (this.opProgress = p));
    const existing = await stIpc.drives().catch(() => []);
    this.drives = existing;
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
    // Partial results are browsable: refresh about once a second while scanning, and once at the end.
    if (finished || now - this.lastReload > 1000) {
      this.lastReload = now;
      this.version++;
      void this.reload();
    }
  }

  async start(root: string) {
    this.error = '';
    try {
      const id = await stIpc.start(root);
      this.sessionId = id;
      this.current = 0;
      this.detail = null;
      this.rows = [];
      this.total = 0;
      this.selected = new Set();
      this.tab = 'explorer';
      this.lastReload = 0;
      this.version++;
      if (!this.sessions.some((s) => s.id === id)) {
        this.sessions = [...this.sessions, await stIpc.progress(id)];
      }
    } catch (e) {
      this.error = String(e);
    }
  }

  async select(id: number) {
    this.sessionId = id;
    this.current = 0;
    this.selected = new Set();
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
      // A fresh scan starts on the Explorer, but refreshing from another tab should stay where it is.
      const tab = this.tab;
      await this.close(p.id);
      await this.start(p.root);
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
      this.desc = key !== 'name';
    }
    void this.reload();
  }

  setText(t: string) {
    this.text = t;
    if (this.reloadTimer) clearTimeout(this.reloadTimer);
    this.reloadTimer = setTimeout(() => void this.reload(), 200);
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
      const present = new Set(page.items.map((r) => r.id));
      this.selected = new Set([...this.selected].filter((s) => present.has(s)));
    } catch (e) {
      this.error = String(e);
    }
  }

  clickRow(index: number, mods: { ctrl: boolean; shift: boolean }) {
    const r = nextSelection(this.selected, this.anchor, this.rows.map((x) => x.id), index, mods);
    this.selected = r.selected;
    this.anchor = r.anchor;
  }

  selectAll() {
    this.selected = new Set(this.rows.map((r) => r.id));
  }

  // ---- file operations: plan, confirm, run ---------------------------------------------------

  say(text: string) {
    this.notice = text;
    if (this.noticeTimer) clearTimeout(this.noticeTimer);
    this.noticeTimer = setTimeout(() => (this.notice = ''), 7000);
  }

  get selectedRows(): StNode[] {
    return this.rows.filter((r) => this.selected.has(r.id));
  }

  async plan(kind: StOpKind, nodes: number[], dest?: string, newName?: string): Promise<StPlan | null> {
    const id = this.sessionId;
    if (id == null || nodes.length === 0) return null;
    try {
      return await stIpc.opPlan(id, kind, nodes, dest, newName);
    } catch (e) {
      this.error = String(e);
      return null;
    }
  }

  /** Runs a plan the app made. `typed` is what the person typed when the plan asked for it. */
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
      const verb = {
        recycle: 'Moved to the Recycle Bin',
        delete: 'Deleted',
        copy: 'Copied',
        move: 'Moved',
        rename: 'Renamed',
        batch_rename: 'Renamed',
        zip: 'Added to a ZIP file',
      }[kind];
      this.say(`${verb}: ${done} ${done === 1 ? 'item' : 'items'}.`);
    }
  }

  /** Shows what a plan would do and waits for the person to confirm. */
  show(plan: StPlan) {
    this.dialog = plan.item_count === 0 ? { type: 'refused', plan } : { type: 'confirm', plan, busy: false, error: '' };
  }

  /** Delete key (to the Recycle Bin) or Shift+Delete (permanent, always asks). `nodes` lets other tabs act on their own list. */
  async remove(permanent: boolean, nodes: number[] = [...this.selected]) {
    const plan = await this.plan(permanent ? 'delete' : 'recycle', nodes);
    if (!plan) return;
    // The Recycle Bin can be undone in Windows, so it needs no extra click unless something is unusual.
    if (!permanent && plan.item_count > 0 && plan.refused.length === 0 && plan.warnings.length === 0) {
      await this.runPlan(plan, false);
      return;
    }
    this.show(plan);
  }

  copySelection(cut: boolean) {
    const nodes = [...this.selected];
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

  /** "Copy to…" and "Move to…": choose a folder first. */
  async sendTo(kind: 'copy' | 'move') {
    const nodes = [...this.selected];
    if (nodes.length === 0) return;
    const dest = await pickFolder({ directory: true, multiple: false, title: kind === 'copy' ? 'Copy to…' : 'Move to…' });
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

  /** Opens the batch rename window for the chosen items. */
  startBatchRename(nodes: number[] = [...this.selected]) {
    if (nodes.length > 0) this.dialog = { type: 'batchrename', nodes };
  }

  startZip(nodes: number[] = [...this.selected]) {
    if (nodes.length > 0) this.dialog = { type: 'zip', nodes, error: '' };
  }

  startExport() {
    if (this.sessionId != null) this.dialog = { type: 'export', error: '' };
  }

  /** Writes the full paths of the chosen items to a text file the person picks. */
  async savePaths(nodes: number[] = [...this.selected]) {
    const id = this.sessionId;
    if (id == null || nodes.length === 0) return;
    const path = await saveFile({ title: 'Save the list of paths', defaultPath: 'paths.txt', filters: [{ name: 'Text file', extensions: ['txt'] }] });
    if (typeof path !== 'string') return;
    try {
      const n = await stTools.exportPaths(id, nodes, path);
      this.say(`Saved ${n} ${n === 1 ? 'path' : 'paths'}.`);
    } catch (e) {
      this.error = String(e);
    }
  }

  async showProperties() {
    const first = this.selectedRows[0]?.id ?? this.current;
    const id = this.sessionId;
    if (id == null) return;
    const d = await stIpc.node(id, first);
    this.dialog = { type: 'props', node: d.node, path: d.path };
  }

  async openSelected(reveal: boolean) {
    const id = this.sessionId;
    const first = this.selectedRows[0];
    if (id == null || !first) return;
    try {
      await stIpc.open(id, first.id, reveal);
    } catch (e) {
      this.error = String(e);
    }
  }

  async copyPath() {
    const id = this.sessionId;
    if (id == null || this.selected.size === 0) return;
    const paths = await stIpc.paths(id, [...this.selected]);
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
