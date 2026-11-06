import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Theme = 'system' | 'dark' | 'light';

export interface Settings {
  version: number;
  general: { close_to_tray: boolean; tray_notice_shown: boolean };
  appearance: { theme: Theme; sidebar_collapsed: boolean };
  shortcuts: { quick_download: string; open_app: string; keep_ready: boolean };
  browser: { launch_from_browser: boolean };
  system: { shell_menu: boolean; autostart: boolean; first_run_done: boolean };
  organizer: {
    dest_root: string | null;
    look_inside_folders: number;
    conflict: 'skip' | 'keep_both';
    auto_enabled: boolean;
    link_downloads: boolean;
  };
  downloads: {
    max_concurrent: number;
    output_dir: string | null;
    allow_duplicates: boolean;
    use_firefox_cookies: boolean;
    notify: boolean;
    default_kind: Kind;
    default_video: Container;
    default_audio: Container;
    sort_into_folders: boolean;
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
  launchFromBrowser?: boolean;
  organizerDest?: string;
  lookInsideFolders?: number;
  organizerConflict?: 'skip' | 'keep_both';
  autoOrganize?: boolean;
  linkDownloads?: boolean;
  defaultKind?: Kind;
  defaultVideo?: Container;
  defaultAudio?: Container;
  sortIntoFolders?: boolean;
  shellMenu?: boolean;
  autostart?: boolean;
  firstRunDone?: boolean;
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

  defaultChoice: () => invoke<Choice>('dl_default_choice'),
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

export interface BrowserStatus {
  state: 'not_connected' | 'needs_repair' | 'connected';
  detail: string | null;
  helper_found: boolean;
  launch_from_browser: boolean;
}

export interface OrgMove {
  src: string;
  dst: string;
  category: string;
  cross_volume: boolean;
}
export type SkipCode =
  | 'link' | 'hidden' | 'cloud' | 'in_progress' | 'recently_changed' | 'locked' | 'unknown' | 'mismatch'
  | 'program' | 'archive' | 'empty' | 'software_folder' | 'folder_not_opened' | 'protected' | 'conflict'
  | 'cross_volume' | 'already_organized' | 'unreadable' | 'excluded' | 'too_many';
export interface OrgSkip {
  path: string;
  is_dir: boolean;
  code: SkipCode;
  message: string;
}
export interface OrgScan {
  plan_id: string;
  source: string;
  dest_root: string;
  moves: OrgMove[];
  skips: OrgSkip[];
  total_moves: number;
  total_skips: number;
  cross_volume_count: number;
}
export interface OrgProblem {
  path: string;
  message: string;
}
export interface OrgReport {
  batch_id: string;
  moved: number;
  failed: OrgProblem[];
  stopped: string | null;
}
export interface OrgUndoReport {
  restored: number;
  skipped: OrgProblem[];
}
export interface OrgBatch {
  id: string;
  mode: string;
  source_root: string;
  dest_root: string;
  created_at: number;
  state: string;
  moved: number;
  undone: number;
  failed: number;
}

export const orgIpc = {
  defaults: () => invoke<{ source: string; dest_root: string; look_inside_folders: number }>('org_defaults'),
  scan: (source: string | null, depth: number | null) => invoke<OrgScan>('org_scan', { source, depth }),
  execute: (planId: string, allowCrossVolume: boolean) =>
    invoke<OrgReport>('org_execute', { planId, allowCrossVolume }),
  cancel: () => invoke<void>('org_cancel'),
  undo: (batchId: string, allowCrossVolume: boolean) =>
    invoke<OrgUndoReport>('org_undo', { batchId, allowCrossVolume }),
  history: () => invoke<OrgBatch[]>('org_history'),
};
export function onOrganizerChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('organizer://changed', () => handler());
}

export const browserIpc = {
  status: () => invoke<BrowserStatus>('browser_status'),
  connect: () => invoke<void>('browser_connect'),
  disconnect: () => invoke<void>('browser_disconnect'),
};
export function onBrowserOpen(handler: (url: string) => void): Promise<UnlistenFn> {
  return listen<string>('browser://open', (e) => handler(e.payload));
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

// ---- File Tools ----------------------------------------------------------------------------
export type FtKind = 'image' | 'video' | 'audio' | 'text' | 'office';
export type FtTarget =
  | 'png' | 'jpg' | 'webp' | 'bmp' | 'tiff' | 'gif' | 'ico' | 'pdf' | 'html'
  | 'mp4' | 'webm' | 'mkv' | 'no_audio' | 'mp3' | 'flac' | 'wav' | 'ogg' | 'opus' | 'm4a';
export interface FtOptions {
  image_quality: number;
  video_quality: 'small' | 'balanced' | 'high';
  max_width: number | null;
  max_height: number | null;
  keep_metadata: boolean;
  background: [number, number, number];
  fps: number | null;
  start_secs: number | null;
  end_secs: number | null;
  audio_kbps: number | null;
  pdf_page: 'fit' | 'a4';
  combine: boolean;
}
export interface FtConversion {
  kind: FtKind;
  target: FtTarget;
  label: string;
  backend: 'image' | 'pdf' | 'ffmpeg' | 'office';
  lossiness: 'lossless' | 'lossy' | 'copy_when_possible';
  available: boolean;
  needs: string | null;
}
export interface FtPreset {
  id: string;
  name: string;
  description: string;
  kind: FtKind;
  target: FtTarget;
  options: FtOptions;
}
export interface FtFile {
  path: string;
  name: string;
  kind: FtKind | null;
  size: number;
  problem: string | null;
}
export interface FtOutcome {
  src: string;
  status: 'done' | 'failed' | 'cancelled';
  dst: string | null;
  message: string | null;
  notes: string[];
  in_bytes: number;
  out_bytes: number;
  sources: number;
}
export type FtEvent =
  | { type: 'started'; index: number; src: string }
  | { type: 'progress'; index: number; fraction: number }
  | { type: 'finished'; index: number; outcome: FtOutcome };

export const ftIpc = {
  info: () => invoke<{ conversions: FtConversion[]; presets: FtPreset[] }>('ft_info'),
  inspect: (paths: string[]) => invoke<{ files: FtFile[]; conversions: FtConversion[] }>('ft_inspect', { paths }),
  run: (args: { paths: string[]; target: FtTarget; options: FtOptions; out_dir: string | null }) =>
    invoke<FtOutcome[]>('ft_run', { args }),
  cancel: () => invoke<void>('ft_cancel'),
  takeInbox: () => invoke<string[]>('ft_take_inbox'),
  open: (path: string, reveal: boolean) => invoke<void>('ft_open', { path, reveal }),
};
export function onFileToolsEvent(handler: (e: FtEvent) => void): Promise<UnlistenFn> {
  return listen<FtEvent>('filetools://event', (e) => handler(e.payload));
}
export function onFileToolsInbox(handler: () => void): Promise<UnlistenFn> {
  return listen('filetools://inbox', () => handler());
}

// ---- Windows integration ---------------------------------------------------------------------
export interface IntegrationStatus {
  shell: { enabled: boolean; mode: 'modern' | 'classic' | 'none'; note: string | null };
  autostart: boolean;
  has_modern_package: boolean;
}
export const integrationIpc = {
  status: () => invoke<IntegrationStatus>('integration_status'),
  repair: () => invoke<IntegrationStatus>('integration_repair'),
};
