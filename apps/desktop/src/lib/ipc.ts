import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

export type Theme = 'system' | 'dark' | 'light';

export interface Settings {
  version: number;
  general: { close_to_tray: boolean; tray_notice_shown: boolean };
  appearance: { theme: Theme; sidebar_collapsed: boolean };
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
  state: 'queued' | 'running' | 'done' | 'failed' | 'cancelled';
  progress: number | null;
  speed_bps: number | null;
  eta_secs: number | null;
  detail: string | null;
}

export const ipc = {
  listModules: () => invoke<ModuleManifest[]>('list_modules'),
  getSettings: () => invoke<Settings>('get_settings'),
  updateSettings: (patch: {
    closeToTray?: boolean;
    theme?: Theme;
    sidebarCollapsed?: boolean;
  }) => invoke<Settings>('update_settings', { patch }),
  listTasks: () => invoke<TaskInfo[]>('list_tasks'),
  appReady: (ms: number) => invoke<void>('app_ready', { ms }),
};

export function onTasks(handler: (batch: TaskInfo[]) => void): Promise<UnlistenFn> {
  return listen<TaskInfo[]>('tasks://batch', (e) => handler(e.payload));
}
