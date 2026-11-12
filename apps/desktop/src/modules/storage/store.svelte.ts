import { stIpc, onStorageProgress, type StDetail, type StDrive, type StNode, type StProgress, type StSort } from '../../lib/ipc';
import { isBusy, nextSelection } from './logic';

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
  tab = $state<'explorer' | 'rankings'>('explorer');
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
      await this.close(p.id);
      await this.start(p.root);
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

  get truncated(): boolean {
    return this.total > this.rows.length;
  }
}

export const storage = new StorageStore();
