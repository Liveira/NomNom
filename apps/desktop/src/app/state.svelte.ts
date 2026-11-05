import { ipc, type Settings, type SettingsPatch, type TaskInfo, type Theme } from '../lib/ipc';
import { buildNav, type NavItem } from '../lib/modules';
import { discoverModules } from './discover';

class AppState {
  nav = $state<NavItem[]>([]);
  settings = $state<Settings | null>(null);
  current = $state('home');
  tasks = $state<Record<string, TaskInfo>>({});

  get activeTasks(): TaskInfo[] {
    return Object.values(this.tasks).filter((t) => t.state === 'running' || t.state === 'queued');
  }

  async init() {
    const [manifests, settings, tasks] = await Promise.all([
      ipc.listModules(),
      ipc.getSettings(),
      ipc.listTasks(),
    ]);
    this.nav = buildNav(discoverModules(), manifests);
    this.settings = settings;
    this.tasks = Object.fromEntries(tasks.map((t) => [t.id, t]));
    this.applyTheme(settings.appearance.theme);
  }

  mergeTasks(batch: TaskInfo[]) {
    const next = { ...this.tasks };
    for (const t of batch) next[t.id] = t;
    this.tasks = next;
  }

  go(id: string) {
    this.current = id;
  }

  async setTheme(theme: Theme) {
    this.settings = await ipc.updateSettings({ theme });
    this.applyTheme(theme);
  }

  /** Applies a settings change and keeps the local copy in sync with what was saved. */
  async update(patch: SettingsPatch) {
    this.settings = await ipc.updateSettings(patch);
  }

  async setCloseToTray(value: boolean) {
    this.settings = await ipc.updateSettings({ closeToTray: value });
  }

  async toggleSidebar() {
    const collapsed = !this.settings?.appearance.sidebar_collapsed;
    this.settings = await ipc.updateSettings({ sidebarCollapsed: collapsed });
  }

  private applyTheme(theme: Theme) {
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    document.documentElement.dataset.theme = theme === 'system' ? (dark ? 'dark' : 'light') : theme;
  }
}

export const app = new AppState();
