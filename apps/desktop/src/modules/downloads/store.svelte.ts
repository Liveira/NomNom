import { app } from '../../app/state.svelte';
import {
  ipc,
  onDownloadsChanged,
  onToolProgress,
  type JobRow,
  type ToolStatus,
} from '../../lib/ipc';

export interface LiveProgress {
  fraction: number | null;
  speedBps: number | null;
  etaSecs: number | null;
  /** True once speed has been stable long enough for the estimate to mean something. */
  etaReliable: boolean;
}

const ETA_AFTER_MS = 5000;

class DownloadsStore {
  jobs = $state<JobRow[]>([]);
  tools = $state<{ ytdlp: ToolStatus; ffmpeg: ToolStatus } | null>(null);
  installing = $state<{ tool: string; done: number; total: number | null } | null>(null);
  installError = $state<string | null>(null);
  /** A link sent from the browser's "Choose in NomNom" menu, waiting to be looked up. */
  pendingUrl = $state<string | null>(null);

  private firstSeen = new Map<string, number>();
  private timer: ReturnType<typeof setTimeout> | undefined;
  private started = false;

  get active(): JobRow[] {
    return this.jobs
      .filter((j) => j.status === 'queued' || j.status === 'running' || j.status === 'paused')
      .sort((a, b) => a.created_at - b.created_at);
  }

  get history(): JobRow[] {
    return this.jobs.filter((j) => j.status === 'done' || j.status === 'failed' || j.status === 'cancelled');
  }

  async start() {
    if (this.started) return;
    this.started = true;
    await Promise.all([this.refresh(), this.refreshTools()]);
    await onDownloadsChanged(() => this.refreshSoon());
    await onToolProgress((p) => {
      this.installing = p;
    });
  }

  refreshSoon() {
    clearTimeout(this.timer);
    this.timer = setTimeout(() => void this.refresh(), 100);
  }

  async refresh() {
    this.jobs = await ipc.listDownloads();
    const live = new Set(this.jobs.map((j) => j.id));
    for (const id of this.firstSeen.keys()) if (!live.has(id)) this.firstSeen.delete(id);
  }

  async refreshTools() {
    this.tools = await ipc.toolsStatus();
  }

  async install(tool: 'ytdlp' | 'ffmpeg') {
    this.installError = null;
    this.installing = { tool, done: 0, total: null };
    try {
      await ipc.installTool(tool);
    } catch (e) {
      this.installError = String(e);
    } finally {
      this.installing = null;
      await this.refreshTools();
    }
  }

  progressFor(jobId: string): LiveProgress | null {
    const task = Object.values(app.tasks)
      .filter((t) => t.ref_id === jobId && t.state === 'running')
      .at(-1);
    if (!task) {
      this.firstSeen.delete(jobId);
      return null;
    }
    if (!this.firstSeen.has(jobId)) this.firstSeen.set(jobId, Date.now());
    const stable = Date.now() - (this.firstSeen.get(jobId) ?? Date.now()) > ETA_AFTER_MS;
    return {
      fraction: task.progress,
      speedBps: task.speed_bps,
      etaSecs: task.eta_secs,
      etaReliable: stable && task.eta_secs != null,
    };
  }
}

export const downloads = new DownloadsStore();
