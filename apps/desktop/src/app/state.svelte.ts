import { ipc, onUpdateAvailable, updateIpc, type UpdateAvailable, type Settings, type SettingsPatch, type TaskInfo, type Theme, type Motion } from '../lib/ipc';
import { buildNav, type NavItem } from '../lib/modules';
import { i18n } from '../lib/i18n/index.svelte';
import { isLight } from '../lib/themes';
import { sidebarItems } from '../lib/tools';
import { discoverModules } from './discover';

class AppState {
  nav = $state<NavItem[]>([]);
  settings = $state<Settings | null>(null);
  current = $state('home');
  tasks = $state<Record<string, TaskInfo>>({});
  /** A newer NomNom found by the last update check. */
  newVersion = $state<UpdateAvailable | null>(null);

  /** The pages in the sidebar: the tools the person picked on the Tools page, plus the core pages. */
  get sidebarNav(): NavItem[] {
    return sidebarItems(this.nav, this.settings?.appearance.sidebar_tools ?? null);
  }

  async setSidebarTools(tools: string[]) {
    this.settings = await ipc.updateSettings({ sidebarTools: tools });
  }

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
    this.watch();
    try {
      this.newVersion = await updateIpc.known();
      await onUpdateAvailable((u) => (this.newVersion = u));
    } catch {
      /* the update notice is optional */
    }
  }

  mergeTasks(batch: TaskInfo[]) {
    const next = { ...this.tasks };
    for (const t of batch) next[t.id] = t;
    this.tasks = next;
  }

  /** Pages visited, so the title bar's back and forward arrows work like a browser's. */
  private trail = $state<string[]>(['home']);
  private at = $state(0);

  get canGoBack(): boolean {
    return this.at > 0;
  }
  get canGoForward(): boolean {
    return this.at < this.trail.length - 1;
  }

  go(id: string) {
    if (id === this.current) return;
    this.trail = [...this.trail.slice(0, this.at + 1), id];
    this.at = this.trail.length - 1;
    this.current = id;
  }

  back() {
    if (!this.canGoBack) return;
    this.at -= 1;
    this.current = this.trail[this.at];
  }

  forward() {
    if (!this.canGoForward) return;
    this.at += 1;
    this.current = this.trail[this.at];
  }

  /**
   * Switches the theme. The change spreads from `from` (where the person clicked) like a ripple,
   * unless motion is off or the system asks for less motion.
   */
  async setTheme(theme: Theme, from?: { x: number; y: number }) {
    const canAnimate =
      typeof document.startViewTransition === 'function' &&
      this.motion !== 'off' &&
      !window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    this.settings = this.settings ? { ...this.settings, appearance: { ...this.settings.appearance, theme } } : this.settings;
    if (canAnimate) {
      const x = from?.x ?? window.innerWidth / 2;
      const y = from?.y ?? window.innerHeight / 2;
      const radius = Math.hypot(Math.max(x, window.innerWidth - x), Math.max(y, window.innerHeight - y));
      const t = document.startViewTransition(() => this.applyTheme(theme));
      void t.ready.then(() =>
        document.documentElement.animate(
          { clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`] },
          { duration: 650, easing: 'cubic-bezier(0.2, 0.85, 0.2, 1)', pseudoElement: '::view-transition-new(root)' },
        ),
      );
    } else {
      this.applyTheme(theme);
    }
    this.settings = await ipc.updateSettings({ theme });
  }

  async setLanguage(language: 'auto' | 'en' | 'pt' | 'es') {
    this.settings = this.settings ? { ...this.settings, general: { ...this.settings.general, language } } : this.settings;
    i18n.apply(language);
    this.settings = await ipc.updateSettings({ language });
  }

  async setMotion(motion: Motion) {
    this.settings = this.settings ? { ...this.settings, appearance: { ...this.settings.appearance, motion } } : this.settings;
    this.applyMotion(motion);
    this.settings = await ipc.updateSettings({ motion });
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

  /** The palette on screen: "system" is dark or light depending on the operating system. */
  palette = $state<string>('dark');

  get motion(): Motion {
    return this.settings?.appearance.motion ?? 'full';
  }

  private applyTheme(theme: Theme) {
    const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
    const palette = theme === 'system' ? (dark ? 'dark' : 'light') : theme;
    const root = document.documentElement;
    root.dataset.theme = palette;
    root.dataset.scheme = isLight(palette) ? 'light' : 'dark';
    this.palette = palette;
  }

  private applyMotion(motion: Motion) {
    document.documentElement.dataset.motion = motion;
  }

  /** Called once at start: sets the theme and motion, and keeps "system" and "hidden" up to date. */
  private watch() {
    this.applyMotion(this.motion);
    window
      .matchMedia('(prefers-color-scheme: dark)')
      .addEventListener('change', () => this.settings && this.applyTheme(this.settings.appearance.theme));
    // Nothing needs to move while the window is hidden or another window is in front of it.
    const idle = () => (document.documentElement.dataset.hidden = document.hidden || !document.hasFocus() ? 'true' : 'false');
    document.addEventListener('visibilitychange', idle);
    window.addEventListener('blur', idle);
    window.addEventListener('focus', idle);
    idle();
  }
}

export const app = new AppState();
