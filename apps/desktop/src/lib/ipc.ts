import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Theme = 'system' | 'dark' | 'light';

export interface Settings {
  version: number;
  general: { close_to_tray: boolean; tray_notice_shown: boolean };
  appearance: { theme: Theme; sidebar_collapsed: boolean };
  shortcuts: { quick_download: string; open_app: string; keep_ready: boolean };
  downloads: {
    max_concurrent: number;
    output_dir: string | null;
    allow_duplicates: boolean;
    use_firefox_cookies: boolean;
    notify: boolean;
  };
}

export interface SettingsPatch {
  closeToTray?: boolean;
  theme?: Theme;
  sidebarCollapsed?: boolean;
  maxConcurrent?: number;
  outputDir?: string;
  allowDuplicates?: boolean;
  firefoxCookies?: boolean;
  notifyDownloads?: boolean;
  quickKeepReady?: boolean;
}

export interface ModuleManifest {
  id: string;
  name: string;
  nav: { order: number; icon: string } | null;
  settings_key: string | null;
}

export interface TaskInfo {
  id: string;
  module: string;
  title: string;
  ref_id: string | null;
  state: 'queued' | 'running' | 'done' | 'failed' | 'cancelled';
  progress: number | null;
  speed_bps: number | null;
  eta_secs: number | null;
  detail: string | null;
}

export type Kind = 'video' | 'audio';
export type Container =
  | 'original' | 'mp4' | 'webm' | 'mkv'
  | 'mp3' | 'm4a' | 'wav' | 'flac' | 'ogg' | 'opus';

export interface Choice {
  kind: Kind;
  container: Container;
  /** null means best available */
  height: number | null;
}

export interface MediaInfo {
  extractor: string;
  title: string;
  thumbnail: string | null;
  webpage_url: string;
  uploader: string | null;
  duration_secs: number | null;
  is_live: boolean;
  heights: number[];
  size_by_height: Record<string, number>;
  audio_size: number | null;
  audio_formats: { ext: string; codec: string | null; bitrate_kbps: number | null }[];
  entries: { title: string | null; url: string }[] | null;
}

export interface Analysis {
  info: MediaInfo;
  site: string;
  known_site: boolean;
}

export type JobStatus = 'queued' | 'running' | 'paused' | 'done' | 'failed' | 'cancelled';

export interface JobRow {
  id: string;
  url: string;
  title: string;
  source: string;
  kind: string;
  format: string;
  quality: string;
  status: JobStatus;
  dest_path: string | null;
  bytes: number | null;
  error: string | null;
  created_at: number;
  finished_at: number | null;
}

export type EnqueueOutcome =
  | { outcome: 'queued'; job_id: string }
  | { outcome: 'already_active'; job_id: string }
  | { outcome: 'already_downloaded'; job_id: string; path: string };

export interface ToolStatus {
  name: string;
  path: string | null;
  version: string | null;
  managed: boolean;
}

export const ipc = {
  listModules: () => invoke<ModuleManifest[]>('list_modules'),
  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (patch: SettingsPatch) => invoke<Settings>('update_settings', { patch }),
  listTasks: () => invoke<TaskInfo[]>('list_tasks'),
  appReady: (ms: number) => invoke<void>('app_ready', { ms }),
  defaultDownloadDir: () => invoke<string>('default_download_dir'),

  analyze: (url: string) => invoke<Analysis>('dl_analyze', { url }),
  enqueue: (url: string, choice: Choice, title?: string, again?: boolean) =>
    invoke<EnqueueOutcome>('dl_enqueue', { url, choice, title, again }),
  listDownloads: () => invoke<JobRow[]>('dl_list'),
  cancel: (id: string) => invoke<void>('dl_cancel', { id }),
  pause: (id: string) => invoke<void>('dl_pause', { id }),
  resume: (id: string) => invoke<void>('dl_resume', { id }),
  retry: (id: string) => invoke<void>('dl_retry', { id }),
  remove: (id: string) => invoke<void>('dl_remove', { id }),
  clearHistory: () => invoke<void>('dl_clear_history'),
  openDownload: (id: string, reveal: boolean) => invoke<void>('dl_open', { id, reveal }),
  toolsStatus: () => invoke<{ ytdlp: ToolStatus; ffmpeg: ToolStatus }>('tools_status'),
  installTool: (tool: 'ytdlp' | 'ffmpeg') => invoke<void>('tools_install', { tool }),
  updateYtdlp: () => invoke<string>('tools_update_ytdlp'),
};

export function onTasks(handler: (batch: TaskInfo[]) => void): Promise<UnlistenFn> {
  return listen<TaskInfo[]>('tasks://batch', (e) => handler(e.payload));
}
export function onDownloadsChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('downloads://changed', () => handler());
}
export function onToolProgress(
  handler: (p: { tool: string; done: number; total: number | null }) => void,
): Promise<UnlistenFn> {
  return listen('tools://progress', (e) => handler(e.payload as never));
}

export interface QuickContext {
  url: string;
  site: string;
}
export interface ShortcutInfo {
  id: 'quick_download' | 'open_app';
  accelerator: string;
  error: string | null;
}

export const quickIpc = {
  context: () => invoke<QuickContext | null>('quick_context'),
  ready: () => invoke<void>('quick_ready'),
  close: () => invoke<void>('quick_close'),
};
export const shortcutsIpc = {
  status: () => invoke<ShortcutInfo[]>('shortcuts_status'),
  set: (id: ShortcutInfo['id'], accelerator: string) =>
    invoke<void>('set_shortcut', { id, accelerator }),
};
export function onQuickContext(handler: () => void): Promise<UnlistenFn> {
  return listen('quick://context', () => handler());
}
