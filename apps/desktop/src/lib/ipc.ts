import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Theme = 'system' | 'dark' | 'light' | 'aurora' | 'synthwave' | 'sakura' | 'ocean' | 'forest' | 'nebula' | 'sunset' | 'matcha' | 'lavender' | 'ember' | 'frost' | 'matrix' | 'cyberpunk' | 'candy' | 'graphite';
export type Motion = 'full' | 'calm' | 'off';

export interface Settings {
  version: number;
  general: { close_to_tray: boolean; tray_notice_shown: boolean; language: 'auto' | 'en' | 'pt' | 'es'; renderer: 'auto' | 'gpu' | 'cpu'; pretty_notifications: boolean };
  appearance: { theme: Theme; motion: Motion; sidebar_collapsed: boolean; sidebar_tools: string[] | null };
  shortcuts: { quick_download: string; open_app: string; clipboard: string; translate_replace: string; translate_popup: string; capture_text: string; launcher: string; launcher_on: boolean; notes: string; pin_top: string; pick_color: string; paste_plain: string; clean_link: string; screenshot: string; keep_ready: boolean };
  actions: CustomAction[];
  clipboard: { enabled: boolean; keep_days: number };
  mini: { clean_links_auto: boolean };
  game: { enabled: boolean; silence: boolean; power_plan: boolean; close: string[]; restore_plan: string | null };
  text: { enabled: boolean; target: string; ocr: boolean; ocr_translate: boolean };
  phone: { save_dir: string | null; idle_minutes: number };
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
    subtitles: boolean;
    subtitle_langs: string;
  };
  storage: {
    snapshot_depth: number;
    big_file_mb: number;
    schedule: 'off' | 'on_start' | 'daily' | 'weekly';
    schedule_roots: string[];
    keep_scheduled: number;
  };
  updates: { auto_check: boolean; last_checked: number };
  monitor: { tray_tooltip: boolean; disk_alerts: boolean };
  activity: { goal_reminder_hour: number; weekly_report: boolean; enabled: boolean; idle_timeout_secs: number; track_nomnom: boolean };
}

export interface SettingsPatch {
  closeToTray?: boolean;
  theme?: Theme;
  motion?: Motion;
  language?: 'auto' | 'en' | 'pt' | 'es';
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
  storageSchedule?: 'off' | 'on_start' | 'daily' | 'weekly';
  storageRoots?: string[];
  storageKeep?: number;
  storageDepth?: number;
  storageBigMb?: number;
  autoUpdateCheck?: boolean;
  subtitles?: boolean;
  subtitleLangs?: string;
  monitorTray?: boolean;
  activityEnabled?: boolean;
  activityIdleSecs?: number;
  activityTrackNomnom?: boolean;
  sidebarTools?: string[];
  clipboardEnabled?: boolean;
  clipboardKeepDays?: number;
  textEnabled?: boolean;
  textTarget?: string;
  textOcr?: boolean;
  textOcrTranslate?: boolean;
  phoneDir?: string;
  phoneIdleMinutes?: number;
  launcherOn?: boolean;
  cleanLinksAuto?: boolean;
  diskAlerts?: boolean;
  gameEnabled?: boolean;
  gameSilence?: boolean;
  gamePowerPlan?: boolean;
  gameClose?: string;
  goalReminderHour?: number;
  weeklyReport?: boolean;
  renderer?: 'auto' | 'gpu' | 'cpu';
  prettyNotifications?: boolean;
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
  id: 'quick_download' | 'open_app' | 'clipboard' | 'translate_replace' | 'translate_popup' | 'capture_text' | 'launcher' | 'notes' | 'pin_top' | 'pick_color' | 'paste_plain' | 'clean_link' | 'screenshot';
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

export interface PlayGoal {
  id: string;
  scope: string;
  kind: 'limit' | 'target';
  period: 'day' | 'week';
  minutes: number;
  enabled: boolean;
}
export interface GoalView {
  goal: PlayGoal;
  name: string;
  icon_url: string | null;
  progress: { used_secs: number; goal_secs: number; percent: number; state: 'under' | 'near' | 'over' | 'behind' | 'met' };
}
export interface WeekReport {
  week_start: string;
  total_secs: number;
  previous_total_secs: number;
  days: number[];
  top_apps: { name: string; secs: number }[];
  goals: { scope: string; name: string; kind: 'limit' | 'target'; period: 'day' | 'week'; minutes: number; kept: number; out_of: number }[];
}
export const goalsIpc = {
  list: () => invoke<GoalView[]>('goals_list'),
  save: (goal: PlayGoal) => invoke<PlayGoal>('goals_save', { goal }),
  remove: (id: string) => invoke<void>('goals_delete', { id }),
  report: (weekStart?: string) => invoke<WeekReport | null>('goals_report', { weekStart: weekStart ?? null }),
  thisWeek: () => invoke<WeekReport>('goals_this_week'),
};
export function onGoalsChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('activity://goals', () => handler());
}

export interface ClipEntry {
  id: number;
  preview: string;
  chars: number;
  pinned: boolean;
  copies: number;
  last_at: number;
}
export const clipIpc = {
  list: (query: string, limit = 200) => invoke<ClipEntry[]>('cb_list', { query, limit }),
  count: () => invoke<number>('cb_count'),
  pin: (id: number, pinned: boolean) => invoke<void>('cb_pin', { id, pinned }),
  remove: (id: number) => invoke<void>('cb_delete', { id }),
  clear: (keepPinned: boolean) => invoke<void>('cb_clear', { keepPinned }),
  use: (id: number, paste: boolean) => invoke<void>('cb_use', { id, paste }),
  text: (id: number) => invoke<string>('cb_text', { id }),
  cleanLink: (text: string) => invoke<string | null>('links_clean', { text }),
  ready: () => invoke<void>('cb_ready'),
  close: () => invoke<void>('cb_close'),
  open: () => invoke<void>('cb_open'),
};
export function onClipboardChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('clipboard://changed', () => handler());
}

export interface KeyStatus {
  set: boolean;
  used: number | null;
  limit: number | null;
  free: boolean;
}
export interface TrLanguage {
  code: string;
  name: string;
}
export interface Translated {
  text: string;
  detected: string;
}
export interface PopupData {
  original: string;
  translated: string | null;
  detected: string;
  target: string;
  error: string | null;
}
export const trIpc = {
  keyStatus: () => invoke<KeyStatus>('tr_key_status'),
  setKey: (key: string) => invoke<KeyStatus>('tr_set_key', { key }),
  clearKey: () => invoke<void>('tr_clear_key'),
  languages: () => invoke<TrLanguage[]>('tr_languages'),
  translate: (text: string) => invoke<Translated>('tr_translate', { text }),
  popupTake: () => invoke<PopupData | null>('tr_popup_take'),
  popupReady: () => invoke<void>('tr_popup_ready'),
  popupClose: () => invoke<void>('tr_popup_close'),
  popupUse: (action: 'copy' | 'replace') => invoke<void>('tr_popup_use', { action }),
};

export interface Subscription {
  id: string;
  url: string;
  title: string;
  kind: 'video' | 'audio';
  every_hours: number;
  enabled: boolean;
  last_checked: number | null;
  started: number;
}
export const subIpc = {
  list: () => invoke<Subscription[]>('sub_list'),
  add: (url: string, kind: 'video' | 'audio', everyHours: number, backfill: number) =>
    invoke<Subscription>('sub_add', { url, kind, everyHours, backfill }),
  remove: (id: string) => invoke<void>('sub_remove', { id }),
  toggle: (id: string, enabled: boolean) => invoke<void>('sub_toggle', { id, enabled }),
  checkNow: (id: string) => invoke<number>('sub_check_now', { id }),
};
export function onSubscriptionsChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('subscriptions://changed', () => handler());
}

export interface PhoneOffered {
  id: string;
  name: string;
  size: number;
}
export interface PhoneReceived {
  name: string;
  path: string;
  size: number;
  at: number;
}
export interface PhoneStatus {
  running: boolean;
  url: string | null;
  qr: string | null;
  dest: string;
  idle_minutes: number;
  offered: PhoneOffered[];
  received: PhoneReceived[];
}
export const phoneIpc = {
  status: () => invoke<PhoneStatus>('ph_status'),
  start: () => invoke<PhoneStatus>('ph_start'),
  stop: () => invoke<PhoneStatus>('ph_stop'),
  offer: (paths: string[]) => invoke<PhoneStatus>('ph_offer', { paths }),
  unoffer: (id: string) => invoke<PhoneStatus>('ph_unoffer', { id }),
};
export function onPhoneChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('phone://changed', () => handler());
}

export interface StartupEffect {
  times: number;
  total_ms: number;
  worst_ms: number;
  last_ms: number;
  last_when: string;
}
export interface StartupItem {
  id: string;
  name: string;
  command: string;
  program: string | null;
  enabled: boolean;
  can_change: boolean;
  from: string;
  icon_url: string | null;
  effect: StartupEffect | null;
}
export interface StartupOverview {
  items: StartupItem[];
  last_boot_ms: number | null;
  has_record: boolean;
}
export const startupIpc = {
  list: () => invoke<StartupOverview>('su_list'),
  set: (id: string, enabled: boolean) => invoke<void>('su_set', { id, enabled }),
};

export type ActionKind = 'app' | 'folder' | 'link' | 'command';
export interface ActionParam {
  name: string;
  label: string;
  default: string;
  kind: 'text' | 'file' | 'folder';
}
export interface CustomAction {
  id: string;
  name: string;
  kind: ActionKind;
  target: string;
  args: string;
  params: ActionParam[];
  shortcut: string | null;
  in_palette: boolean;
  cwd: string | null;
}
export interface ActionView extends CustomAction {
  shortcut_error: string | null;
}
export interface ActionResult {
  ok: boolean;
  code: number | null;
  output: string;
  truncated: boolean;
  timed_out: boolean;
  message: string;
}
export const actionsIpc = {
  list: () => invoke<ActionView[]>('ca_list'),
  save: (action: CustomAction) => invoke<ActionView[]>('ca_save', { action }),
  remove: (id: string) => invoke<ActionView[]>('ca_delete', { id }),
  run: (id: string, values: Record<string, string>) => invoke<ActionResult>('ca_run', { id, values }),
};

export interface Reminder {
  id: number;
  text: string;
  due_at: number;
  repeat: 'daily' | 'weekly' | null;
  done: boolean;
}
export interface ReminderPreview {
  text: string;
  due_at: number;
  repeat: 'daily' | 'weekly' | null;
}
export const reminderIpc = {
  preview: (text: string) => invoke<ReminderPreview | null>('rm_preview', { text }),
  addText: (text: string) => invoke<void>('rm_add_text', { text }),
  addAt: (text: string, dueAt: number, repeat: string | null) => invoke<void>('rm_add_at', { text, dueAt, repeat }),
  list: () => invoke<Reminder[]>('rm_list'),
  remove: (id: number) => invoke<void>('rm_delete', { id }),
  snooze: (id: number, minutes: number) => invoke<void>('rm_snooze', { id, minutes }),
};
export function onRemindersChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('reminders://changed', () => handler());
}

export interface Note {
  id: number;
  text: string;
  pinned: boolean;
  created_at: number;
  updated_at: number;
}
export const noteIpc = {
  list: (query: string) => invoke<Note[]>('nt_list', { query }),
  save: (id: number | null, text: string) => invoke<number>('nt_save', { id, text }),
  pin: (id: number, pinned: boolean) => invoke<void>('nt_pin', { id, pinned }),
  remove: (id: number) => invoke<void>('nt_delete', { id }),
};
export function onNotesChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('notes://changed', () => handler());
}

export type LauncherKind = 'app' | 'folder' | 'recent';
export interface LauncherEntry {
  name: string;
  path: string;
  kind: LauncherKind;
}
export const launcherIpc = {
  entries: () => invoke<LauncherEntry[]>('lc_entries'),
  open: (path: string) => invoke<void>('lc_open', { path }),
  openPath: (path: string, reveal: boolean) => invoke<void>('lc_open_path', { path, reveal }),
  go: (page: string) => invoke<void>('lc_go', { page }),
  close: () => invoke<void>('lc_close'),
  ready: () => invoke<void>('lc_ready'),
};

export interface NoticeItem {
  id: number;
  at: number;
  kind: 'info' | 'success' | 'warning' | 'error';
  title: string;
  body: string;
  action: string | null;
}
export const noticeIpc = {
  list: () => invoke<NoticeItem[]>('nf_list'),
  clear: () => invoke<void>('nf_clear'),
  remove: (id: number) => invoke<void>('nf_delete', { id }),
};
export function onNoticesChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('notifications://changed', () => handler());
}

export type RuleStep =
  | { type: 'move'; folder: string }
  | { type: 'copy'; folder: string }
  | { type: 'rename'; template: string }
  | { type: 'convert'; to: 'webp' | 'jpg' | 'png'; originals: string }
  | { type: 'run'; action: string }
  | { type: 'notify' };
export interface FolderRule {
  id: string;
  name: string;
  enabled: boolean;
  folder: string;
  subfolders: boolean;
  extensions: string[];
  name_has: string;
  min_kb: number;
  max_kb: number;
  steps: RuleStep[];
  since: number;
}
export interface RuleLogItem {
  id: number;
  at: number;
  rule: string;
  step: string;
  from: string;
  to: string;
  ok: boolean;
  note: string;
  undone: boolean;
}
export const rulesIpc = {
  list: () => invoke<FolderRule[]>('fr_list'),
  save: (rule: FolderRule) => invoke<FolderRule[]>('fr_save', { rule }),
  remove: (id: string) => invoke<FolderRule[]>('fr_delete', { id }),
  preview: (id: string) => invoke<{ name: string; size: number }[]>('fr_preview', { id }),
  applyExisting: (id: string) => invoke<number>('fr_apply_existing', { id }),
  log: () => invoke<RuleLogItem[]>('fr_log'),
  undo: (id: number) => invoke<void>('fr_undo', { id }),
  clearLog: () => invoke<void>('fr_clear_log'),
};
export function onRulesChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('rules://changed', () => handler());
}

export const settingsFileIpc = {
  save: (path: string) => invoke<void>('settings_export', { path }),
  load: (path: string) => invoke<Settings>('settings_import', { path }),
};

export interface Snippet {
  id: number;
  name: string;
  body: string;
}
export const snippetIpc = {
  list: () => invoke<Snippet[]>('sn_list'),
  save: (id: number | null, name: string, body: string) => invoke<number>('sn_save', { id, name, body }),
  remove: (id: number) => invoke<void>('sn_delete', { id }),
  preview: (body: string) => invoke<string>('sn_preview', { body }),
  use: (id: number, paste: boolean) => invoke<void>('sn_use', { id, paste }),
};
export function onSnippetsChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('snippets://changed', () => handler());
}

export interface FileHit {
  path: string;
  name: string;
  is_dir: boolean;
  size: number;
  modified: number;
}
export const fileIpc = {
  search: (query: string) => invoke<FileHit[]>('fs_search', { query }),
};

export const gameModeIpc = {
  status: () => invoke<{ on: boolean; game: string | null }>('gm_status'),
};

export const shotIpc = {
  image: () => invoke<string | null>('sh_image'),
  ready: () => invoke<void>('sh_ready'),
  cancel: () => invoke<void>('sh_cancel'),
  pick: (x: number, y: number, w: number, h: number) => invoke<void>('sh_pick', { x, y, w, h }),
  editImage: () => invoke<string | null>('sh_edit_image'),
  editorReady: () => invoke<void>('sh_editor_ready'),
  close: () => invoke<void>('sh_close'),
  copy: (png: string) => invoke<void>('sh_copy', { png }),
  save: (path: string, png: string) => invoke<void>('sh_save', { path, png }),
  start: () => invoke<void>('sh_start'),
};

export const miniIpc = {
  pick: () => invoke<void>('mini_pick'),
  cleanNow: () => invoke<void>('mini_clean_now'),
};
export const pickerIpc = {
  image: () => invoke<string | null>('pk_image'),
  ready: () => invoke<void>('pk_ready'),
  cancel: () => invoke<void>('pk_cancel'),
  done: (hex: string) => invoke<void>('pk_done', { hex }),
};

export const ocrIpc = {
  start: () => invoke<void>('ocr_start'),
  image: () => invoke<string | null>('ocr_image'),
  ready: () => invoke<void>('ocr_ready'),
  cancel: () => invoke<void>('ocr_cancel'),
  run: (x: number, y: number, w: number, h: number) => invoke<void>('ocr_run', { x, y, w, h }),
};

export interface ToastCard {
  kind: 'info' | 'success' | 'warning' | 'error';
  title: string;
  body: string;
  action: string | null;
}
export const toastIpc = {
  take: () => invoke<ToastCard[]>('toast_take'),
  layout: (height: number) => invoke<void>('toast_layout', { height }),
  open: (action: string) => invoke<void>('toast_open', { action }),
  close: () => invoke<void>('toast_close'),
  test: () => invoke<void>('toast_test'),
};

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

// ---- Desktop integration ---------------------------------------------------------------------
export interface IntegrationStatus {
  platform?: 'windows' | 'linux';
  shell: { enabled: boolean; mode: 'modern' | 'classic' | 'linux' | 'none'; note: string | null };
  autostart: boolean;
  has_modern_package: boolean;
}
export const integrationIpc = {
  status: () => invoke<IntegrationStatus>('integration_status'),
  repair: () => invoke<IntegrationStatus>('integration_repair'),
};

// ---- NomNom updates ---------------------------------------------------------------------------
export interface UpdateAvailable {
  version: string;
  notes: string;
  page: string;
  file_name: string;
  url: string;
  size: number;
  sha256: string;
}
export type UpdateCheck =
  | { state: 'up_to_date'; current: string; latest: string }
  | ({ state: 'available' } & UpdateAvailable);
export const updateIpc = {
  version: () => invoke<string>('update_version'),
  known: () => invoke<UpdateAvailable | null>('update_known'),
  check: () => invoke<UpdateCheck>('update_check'),
  install: () => invoke<string | null>('update_install'),
};
export function onUpdateAvailable(handler: (u: UpdateAvailable) => void): Promise<UnlistenFn> {
  return listen<UpdateAvailable>('update://available', (e) => handler(e.payload));
}
export function onUpdateProgress(handler: (p: { done: number; total: number | null }) => void): Promise<UnlistenFn> {
  return listen('update://progress', (e) => handler(e.payload as never));
}

// ---- Disk Analyzer: cleanup ---------------------------------------------------------------------
export interface StCleanSpot {
  id: string;
  title: string;
  why: string;
  path: string;
  size: number;
  files: number;
  partial: boolean;
}
export const cleanupIpc = {
  spots: () => invoke<StCleanSpot[]>('st_cleanup_spots'),
};

// ---- Disk Analyzer ---------------------------------------------------------------------------
export interface StDrive {
  root: string;
  letter: string;
  label: string;
  filesystem: string;
  kind: 'fixed' | 'removable' | 'network' | 'optical' | 'ram' | 'unknown';
  media: 'ssd' | 'hdd' | 'unknown';
  total: number;
  free: number;
  used: number;
  cluster: number;
  serial: number;
}
export type StState = 'running' | 'paused' | 'done' | 'cancelled';
export interface StProgress {
  id: number;
  root: string;
  state: StState;
  files: number;
  dirs: number;
  bytes: number;
  /** Space taken on disk, the number to compare with `drive_used`. */
  allocated: number;
  errors: number;
  elapsed_ms: number;
  elevated?: boolean;
  mft_bytes?: number;
  drive_used: number | null;
  drive_free?: number | null;
  drive_total: number | null;
  drive_fs?: string | null;
}
export interface StNode {
  id: number;
  name: string;
  is_dir: boolean;
  is_link: boolean;
  size: number;
  alloc: number;
  own_size?: number;
  own_alloc?: number;
  own_files?: number;
  files: number;
  dirs: number;
  mtime: number | null;
  ctime: number | null;
  atime: number | null;
  attrs: number;
  flags: number;
  percent: number;
  parent: number | null;
  owner?: string | null;
  ads_count?: number;
  ads_size?: number;
  path_len?: number;
}
export interface StPage {
  total: number;
  items: StNode[];
}
export interface StDetail {
  node: StNode;
  path: string;
  trail: StNode[];
}
export interface StError {
  path: string;
  code: number;
  message: string;
}
export type StSort =
  | 'name' | 'size' | 'alloc' | 'own_size' | 'wasted' | 'files' | 'dirs'
  | 'modified' | 'created' | 'accessed' | 'owner' | 'path_len' | 'type' | 'percent';
export interface StFilter {
  min_size?: number;
  max_size?: number;
  extensions?: string[];
  category?: string;
  modified_after?: number;
  modified_before?: number;
  name_contains?: string;
  owner_contains?: string;
}
export interface StGroup {
  key: string;
  size: number;
  alloc: number;
  files: number;
}
export interface StScanStartOpts {
  elevated?: boolean;
  collect_owners?: boolean;
  collect_streams?: boolean;
  exclude?: string[];
}

export const stIpc = {
  drives: () => invoke<StDrive[]>('st_drives'),
  takeInbox: () => invoke<string[]>('st_take_inbox'),
  start: (root: string, options?: StScanStartOpts) => invoke<number>('st_scan_start', { root, options }),
  saveScan: (id: number, path: string) => invoke<number>('st_scan_save', { id, path }),
  loadScan: (path: string) => invoke<number>('st_scan_load', { path }),
  pause: (id: number) => invoke<void>('st_scan_pause', { id }),
  resume: (id: number) => invoke<void>('st_scan_resume', { id }),
  cancel: (id: number) => invoke<void>('st_scan_cancel', { id }),
  rescan: (id: number, node: number) => invoke<void>('st_rescan', { id, node }),
  close: (id: number) => invoke<void>('st_close', { id }),
  progress: (id: number) => invoke<StProgress>('st_progress', { id }),
  errors: (id: number) => invoke<StError[]>('st_errors', { id }),
  children: (
    id: number,
    node: number,
    sort: StSort,
    desc: boolean,
    offset: number,
    limit: number,
    text?: string,
    only?: 'dirs' | 'files',
  ) => invoke<StPage>('st_children', { id, node, sort, desc, offset, limit, text, only }),
  node: (id: number, node: number) => invoke<StDetail>('st_node', { id, node }),
  topFiles: (id: number, node: number, n: number, filter: StFilter) =>
    invoke<StNode[]>('st_top_files', { id, node, n, filter }),
  topFolders: (id: number, node: number, n: number, mode: 'inclusive' | 'exclusive', minSize?: number) =>
    invoke<StNode[]>('st_top_folders', { id, node, n, mode, minSize }),
  opPlan: (id: number, kind: StOpKind, nodes: number[], dest?: string, newName?: string) =>
    invoke<StPlan>('st_op_plan', { id, kind, nodes, dest, newName }),
  opRun: (planId: string, confirmed: boolean, typed: string | undefined, includeReadonly: boolean) =>
    invoke<StReport>('st_op_run', { planId, confirmed, typed, includeReadonly }),
  opCancel: () => invoke<void>('st_op_cancel'),
  open: (id: number, node: number, reveal: boolean) => invoke<void>('st_open', { id, node, reveal }),
  treemap: (id: number, node: number, depth: number, maxChildren: number) =>
    invoke<StTree>('st_treemap', { id, node, depth, maxChildren }),
  paths: (id: number, nodes: number[]) => invoke<string[]>('st_paths', { id, nodes }),
  stats: (id: number, node: number) =>
    invoke<{ extensions: StGroup[]; categories: StGroup[]; owners: StGroup[]; sizes: StGroup[] }>('st_stats', { id, node }),
};
export function onStorageProgress(handler: (p: StProgress) => void): Promise<UnlistenFn> {
  return listen<StProgress>('storage://progress', (e) => handler(e.payload));
}
export function onStorageInbox(handler: () => void): Promise<UnlistenFn> {
  return listen('storage://inbox', () => handler());
}

export type StOpKind =
  | 'recycle' | 'delete' | 'copy' | 'move' | 'rename' | 'batch_rename' | 'zip'
  | 'compress' | 'decompress' | 'hardlink';
export interface StPlanItem {
  path: string;
  is_dir: boolean;
  size: number;
  files: number;
  warn: boolean;
  new_name: string | null;
  keep: string | null;
}
export interface StPlan {
  plan_id: string;
  kind: StOpKind;
  item_count: number;
  items: StPlanItem[];
  total_size: number;
  total_files: number;
  refused: { path: string; reason: string }[];
  warnings: string[];
  needs_typed: boolean;
  dest: string | null;
  new_name: string | null;
}
export interface StItemResult {
  path: string;
  status: 'done' | 'failed' | 'skipped';
  message: string | null;
  size: number;
  to: string | null;
}
export interface StReport {
  results: StItemResult[];
  cancelled: boolean;
}
export interface StOpProgress {
  done: number;
  total: number;
  current: string;
}

export function onStorageOp(handler: (p: StOpProgress) => void): Promise<UnlistenFn> {
  return listen<StOpProgress>('storage://op', (e) => handler(e.payload));
}
export function onStorageChanged(handler: (id: number) => void): Promise<UnlistenFn> {
  return listen<number>('storage://changed', (e) => handler(e.payload));
}

export interface StTree {
  id: number;
  name: string;
  is_dir: boolean;
  size: number;
  alloc: number;
  files?: number;
  mtime?: number | null;
  ext?: string;
  category: string;
  children: StTree[];
  hidden_count: number;
  hidden_size: number;
  hidden_alloc: number;
}

// ---- Disk Analyzer: search, duplicates, batch, reports ------------------------------------------
export type StTextMode = 'contains' | 'wildcard' | 'regex';
export type StDateField = 'created' | 'modified' | 'accessed';
export type StQuery =
  | { type: 'and'; items: StQuery[] }
  | { type: 'or'; items: StQuery[] }
  | { type: 'not'; item: StQuery }
  | { type: 'name'; text: string; mode: StTextMode }
  | { type: 'path'; text: string; mode: StTextMode }
  | { type: 'owner'; text: string; mode: StTextMode }
  | { type: 'path_len'; min: number | null; max: number | null }
  | { type: 'empty_folder' }
  | { type: 'ext'; values: string[] }
  | { type: 'size'; min: number | null; max: number | null }
  | { type: 'date'; field: StDateField; after: number | null; before: number | null }
  | { type: 'age'; field: StDateField; older_than_days: number | null; newer_than_days: number | null }
  | { type: 'attr'; name: string; set: boolean }
  | { type: 'kind'; category: string }
  | { type: 'item'; what: 'file' | 'folder' };
export interface StFound {
  total: number;
  truncated: boolean;
  items: StNode[];
}
export interface StDupCopy {
  node: number;
  path: string;
  mtime: number | null;
  names: number;
}
export interface StDupGroup {
  size: number;
  sha256: string;
  copies: StDupCopy[];
  recoverable: number;
}
export interface StDupProgress {
  stage: 'listing' | 'comparing' | 'hashing' | 'done' | 'cancelled';
  candidates: number;
  files_done: number;
  files_total: number;
  bytes_done: number;
  bytes_total: number;
  unreadable: number;
  groups: number;
  recoverable: number;
}
export interface StRenameRule {
  find: string;
  replace: string;
  regex: boolean;
  prefix: string;
  suffix: string;
  start: number;
  step: number;
  pad: number;
  case: '' | 'lower' | 'upper';
  include_extension: boolean;
}
export interface StRenameRow {
  old: string;
  new: string | null;
  error: string | null;
}
export type StReportSpec = (
  | { kind: 'tree'; depth: number }
  | { kind: 'top_files'; n: number }
  | { kind: 'top_folders'; n: number; mode: 'inclusive' | 'exclusive' }
  | { kind: 'extensions' }
  | { kind: 'categories' }
  | { kind: 'owners' }
  | { kind: 'sizes' }
  | { kind: 'age'; field: StDateField }
  | { kind: 'duplicates' }
) & { format: 'csv' | 'json' | 'html' | 'xlsx' | 'pdf'; columns: string[] };

export const stTools = {
  search: (id: number, node: number, query: StQuery, limit: number) => invoke<StFound>('st_search', { id, node, query, limit }),
  byAge: (id: number, node: number, field: StDateField) => invoke<StGroup[]>('st_by_age', { id, node, field }),
  dupesStart: (id: number, node: number, minSize: number) => invoke<void>('st_dupes_start', { id, node, minSize }),
  dupesProgress: () => invoke<[number, StDupProgress] | null>('st_dupes_progress'),
  dupesGroups: (offset: number, limit: number) => invoke<StDupGroup[]>('st_dupes_groups', { offset, limit }),
  dupesCancel: () => invoke<void>('st_dupes_cancel'),
  dupesPlan: (id: number, permanent: boolean, groups: { keep: number; remove: number[] }[], hardlink = false) =>
    invoke<StPlan>('st_dupes_plan', { id, permanent, groups, hardlink }),
  renamePreview: (id: number, nodes: number[], rule: StRenameRule) => invoke<StRenameRow[]>('st_rename_preview', { id, nodes, rule }),
  renamePlan: (id: number, nodes: number[], rule: StRenameRule) => invoke<StPlan>('st_rename_plan', { id, nodes, rule }),
  export: (id: number, node: number, spec: StReportSpec, path: string) => invoke<number>('st_export', { id, node, spec, path }),
  exportPaths: (id: number, nodes: number[], path: string) => invoke<number>('st_export_paths', { id, nodes, path }),
};

// ---- Disk Analyzer: history ---------------------------------------------------------------------------
export interface StSnapshot {
  id: number;
  root: string;
  label: string;
  source: 'manual' | 'scheduled';
  created_at: number;
  files: number;
  dirs: number;
  bytes: number;
  alloc: number;
  depth: number;
  big_min: number;
  errors: number;
}
export type StChange = 'grew' | 'shrank' | 'new' | 'gone';
export interface StChangeRow {
  path: string;
  change: StChange;
  before: number | null;
  after: number | null;
  delta: number;
}
export interface StComparison {
  bytes_before: number;
  bytes_after: number;
  delta: number;
  files_before: number;
  files_after: number;
  depth: number;
  big_min: number;
  dirs: StChangeRow[];
  dirs_changed: number;
  files: StChangeRow[];
  files_changed: number;
}
export interface StCompareView {
  before: StSnapshot;
  after: StSnapshot | null;
  comparison: StComparison;
}
export interface StTrendPoint {
  id: number;
  created_at: number;
  size: number | null;
}
export interface StRunNote {
  root: string;
  at: number;
  ok: boolean;
  message: string;
}

export const stHistory = {
  save: (id: number, label: string) => invoke<StSnapshot>('st_snapshot_save', { id, label }),
  list: (root: string) => invoke<StSnapshot[]>('st_snapshots', { root }),
  remove: (snapshot: number) => invoke<void>('st_snapshot_delete', { snapshot }),
  compare: (a: number, b: number | null, id: number | null, limit: number) =>
    invoke<StCompareView>('st_snapshot_compare', { a, b, id, limit }),
  exportComparison: (a: number, b: number | null, id: number | null, format: 'csv' | 'json' | 'html' | 'xlsx' | 'pdf', path: string) =>
    invoke<number>('st_export_comparison', { a, b, id, format, path }),
  trend: (root: string, folder: string) => invoke<StTrendPoint[]>('st_snapshot_trend', { root, folder }),
  scheduleStatus: () => invoke<{ running: string | null; notes: StRunNote[] }>('st_schedule_status'),
};
export function onStorageSnapshots(handler: (root: string) => void): Promise<UnlistenFn> {
  return listen<string>('storage://snapshots', (e) => handler(e.payload));
}
export function onStorageSchedule(handler: () => void): Promise<UnlistenFn> {
  return listen<string>('storage://schedule', () => handler());
}

// ---- Playtime: active foreground app & game time tracking --------------------------------------------
export type ActRange = 'today' | 'week' | 'month' | 'year' | 'all';
export type ActCategory = 'creative' | 'game' | 'browser' | 'social' | 'work' | 'media' | 'general';

export interface ActLiveFocus {
  app_id: string;
  name: string;
  category: string;
  session_secs: number;
  started_at: number;
  is_afk: boolean;
}

export interface ActAppRow {
  id: string;
  name: string;
  custom_name: string | null;
  category: string;
  exe_path: string | null;
  icon_url: string | null;
  hidden: boolean;
  range_secs: number;
  today_secs: number;
  week_secs: number;
  month_secs: number;
  year_secs: number;
  total_secs: number;
  first_seen: number;
  last_seen: number;
  share: number;
}

export interface ActTimelinePoint {
  key: string;
  label: string;
  secs: number;
  by_category: Record<string, number>;
}

export interface ActHeatmapDay {
  day: string;
  secs: number;
}

export interface ActCategoryTotal {
  category: string;
  secs: number;
  apps: number;
}

export interface ActSessionEntry {
  id: number;
  app_id: string;
  app_name: string;
  category: string;
  icon_url: string | null;
  started_at: number;
  ended_at: number;
  active_secs: number;
}

export interface ActOverview {
  enabled: boolean;
  idle_timeout_secs: number;
  track_nomnom: boolean;
  range: ActRange;
  live: ActLiveFocus | null;
  range_secs: number;
  today_secs: number;
  week_secs: number;
  month_secs: number;
  year_secs: number;
  all_time_secs: number;
  daily_avg_secs: number;
  sessions_count: number;
  peak_hour: number | null;
  apps: ActAppRow[];
  timeline: ActTimelinePoint[];
  clock_hours: number[];
  heatmap: ActHeatmapDay[];
  categories: ActCategoryTotal[];
  recent_sessions: ActSessionEntry[];
}

export interface ActAppDetail {
  app: ActAppRow;
  longest_session_secs: number;
  sessions_count: number;
  active_days: number;
  peak_hour: number | null;
  daily_30d: ActTimelinePoint[];
  monthly_12m: ActTimelinePoint[];
  clock_hours: number[];
  recent_sessions: ActSessionEntry[];
}

export const actIpc = {
  live: () => invoke<ActLiveFocus | null>('act_live'),
  overview: (range: ActRange, includeHidden = false) =>
    invoke<ActOverview>('act_overview', { range, includeHidden }),
  appDetail: (appId: string) => invoke<ActAppDetail>('act_app_detail', { appId }),
  updateApp: (
    appId: string,
    patch: { customName?: string; category?: string; hidden?: boolean },
  ) =>
    invoke<void>('act_update_app', {
      appId,
      customName: patch.customName,
      category: patch.category,
      hidden: patch.hidden,
    }),
  deleteApp: (appId: string) => invoke<void>('act_delete_app', { appId }),
};

export function onActivityUpdated(handler: () => void): Promise<UnlistenFn> {
  return listen<null>('activity://updated', () => handler());
}

// ---- Focus mode -------------------------------------------------------------------------------
export interface FocusActive {
  id: number;
  kind: 'focus' | 'break';
  label: string;
  started_at: number;
  planned_secs: number;
  remaining_secs: number;
}
export interface FocusBlock {
  id: number;
  kind: 'focus' | 'break';
  label: string;
  started_at: number;
  ended_at: number;
  planned_secs: number;
  completed: boolean;
  apps: { name: string; secs: number }[];
}
export interface FocusStatus {
  active: FocusActive | null;
  today_blocks: number;
  today_secs: number;
  streak_days: number;
  recent: FocusBlock[];
}
export const focusIpc = {
  status: () => invoke<FocusStatus>('focus_status'),
  start: (kind: 'focus' | 'break', minutes: number, label: string) => invoke<FocusActive>('focus_start', { kind, minutes, label }),
  stop: () => invoke<FocusBlock | null>('focus_stop'),
};
export function onFocusChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('focus://changed', () => handler());
}

// ---- File Tools: PDF tools ----------------------------------------------------------------------
export interface PdfInfo {
  path: string;
  name: string;
  pages: number | null;
  problem: string | null;
}
export const pdfIpc = {
  info: (paths: string[]) => invoke<PdfInfo[]>('pdf_info', { paths }),
  merge: (paths: string[]) => invoke<string>('pdf_merge', { paths }),
  split: (path: string, mode: 'each' | 'pages', pages: string) => invoke<string[]>('pdf_split', { path, mode, pages }),
};

// ---- File Tools: rename many files ----------------------------------------------------------------
export interface RenameOptions {
  template: string;
  find: string;
  replace: string;
  case: 'keep' | 'lower' | 'upper' | 'title';
  spaces: 'keep' | 'underscore' | 'dash' | 'dot' | 'remove';
  order: 'given' | 'name' | 'modified';
  start: number;
  step: number;
  lower_ext: boolean;
  today: string;
}
export interface RenameItem {
  from: string;
  from_name: string;
  to_name: string;
  status: 'ok' | 'same' | 'conflict' | 'invalid';
  note: string;
}
export interface RenameResult {
  renamed: number;
  skipped: number;
  failed: [string, string][];
}
export const renameIpc = {
  inspect: (paths: string[]) => invoke<string[]>('rn_inspect', { paths }),
  preview: (paths: string[], options: RenameOptions) => invoke<RenameItem[]>('rn_preview', { paths, options }),
  apply: (paths: string[], options: RenameOptions) => invoke<RenameResult>('rn_apply', { paths, options }),
  canUndo: () => invoke<number>('rn_can_undo'),
  undo: () => invoke<RenameResult>('rn_undo'),
};

// ---- Downloader: schedule -----------------------------------------------------------------------
export interface Scheduled {
  id: string;
  url: string;
  title: string | null;
  run_at: number;
}
export const scheduleIpc = {
  add: (url: string, choice: Choice, title: string | undefined, runAt: number) =>
    invoke<Scheduled>('dl_schedule', { url, choice, title, runAt }),
  list: () => invoke<Scheduled[]>('dl_scheduled'),
  cancel: (id: string) => invoke<void>('dl_unschedule', { id }),
};
export function onScheduledChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('downloads://scheduled', () => handler());
}

// ---- Backup -------------------------------------------------------------------------------------
export interface BackupPlan {
  id: string;
  name: string;
  sources: string[];
  destination: string;
  schedule: 'manual' | 'hourly' | 'daily' | 'weekly';
  keep: number;
  exclude: string;
  enabled: boolean;
}
export interface BackupLastRun {
  started_at: number;
  ended_at: number;
  ok: boolean;
  files: number;
  bytes: number;
  copied: number;
  message: string | null;
}
export interface BackupPlanView {
  plan: BackupPlan;
  last: BackupLastRun | null;
  next_at: number | null;
  running: boolean;
  problem: string | null;
}
export interface BackupVersion {
  name: string;
  started_at: number;
  files: number;
  bytes: number;
}
export interface BackupTree {
  folders: { name: string; files: number; bytes: number }[];
  files: { name: string; bytes: number; modified: number }[];
}
export interface BackupRestoreReport {
  restored: number;
  bytes: number;
  renamed: number;
  failed: string[];
  cancelled: boolean;
}
export const backupIpc = {
  plans: () => invoke<BackupPlanView[]>('bk_plans'),
  save: (plan: BackupPlan) => invoke<BackupPlan>('bk_save', { plan }),
  remove: (id: string) => invoke<void>('bk_delete', { id }),
  run: (id: string) => invoke<void>('bk_run', { id }),
  cancel: () => invoke<void>('bk_cancel'),
  versions: (id: string) => invoke<BackupVersion[]>('bk_versions', { id }),
  tree: (id: string, version: string, prefix: string) => invoke<BackupTree>('bk_tree', { id, version, prefix }),
  restore: (id: string, version: string, only: string[] | null, target: string) =>
    invoke<BackupRestoreReport>('bk_restore', { id, version, only, target }),
  verify: (id: string, version: string) =>
    invoke<{ checked: number; missing: string[]; wrong_size: string[] }>('bk_verify', { id, version }),
};
export function onBackupChanged(handler: () => void): Promise<UnlistenFn> {
  return listen('backup://changed', () => handler());
}
export function onBackupProgress(handler: (p: { plan_id: string; files: number; bytes: number; current: string }) => void): Promise<UnlistenFn> {
  return listen('backup://progress', (e) => handler(e.payload as never));
}

// ---- System monitor -----------------------------------------------------------------------------
export interface MonProc {
  name: string;
  pid: number;
  cpu: number;
  memory: number;
}
export interface MonSnapshot {
  cpu: number;
  cores: number[];
  cpu_name: string;
  mem_used: number;
  mem_total: number;
  swap_used: number;
  swap_total: number;
  disks: { name: string; mount: string; total: number; used: number; removable: boolean }[];
  net_down: number;
  net_up: number;
  uptime_secs: number;
  temp_c: number | null;
  by_cpu: MonProc[];
  by_memory: MonProc[];
}
export interface HwGpu {
  name: string;
  load: number | null;
  temp_c: number | null;
  mem_used_mb: number | null;
  mem_total_mb: number | null;
  fan_percent: number | null;
  power_w: number | null;
}
export interface HwBattery {
  percent: number;
  plugged: boolean;
  charging: boolean;
  minutes_left: number | null;
}
export interface HwDisk {
  name: string;
  media: string;
  health: string;
  status: string;
  size: number;
  temp_c: number | null;
  wear: number | null;
  hours: number | null;
  read_errors: number | null;
  write_errors: number | null;
  verdict: 'good' | 'watch' | 'bad';
  why: string[];
}
export const hwIpc = {
  status: () => invoke<{ gpus: HwGpu[]; battery: HwBattery | null }>('hw_status'),
  disks: () => invoke<HwDisk[]>('hw_disks'),
};

export const monitorIpc = {
  snapshot: () => invoke<MonSnapshot>('mon_snapshot'),
};
