<script lang="ts">
  import { app } from '../../app/state.svelte';
  import type { Theme } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import DownloaderSettings from './DownloaderSettings.svelte';
  import Shortcuts from './Shortcuts.svelte';

  const themes: { value: Theme; label: string }[] = [
    { value: 'system', label: 'System' },
    { value: 'dark', label: 'Dark' },
    { value: 'light', label: 'Light' },
  ];
  const theme = $derived(app.settings?.appearance.theme ?? 'system');
  const closeToTray = $derived(app.settings?.general.close_to_tray ?? true);
</script>

<h1>Settings</h1>

<section class="group">
  <h3>Appearance</h3>
  <div class="row">
    <div class="label">Theme</div>
    <div class="segmented" role="radiogroup" aria-label="Theme">
      {#each themes as t (t.value)}
        <button
          role="radio"
          aria-checked={theme === t.value}
          class:on={theme === t.value}
          onclick={() => app.setTheme(t.value)}>{t.label}</button
        >
      {/each}
    </div>
  </div>
</section>

<section class="group">
  <h3>General</h3>
  <div class="row">
    <div>
      <div class="label">Keep running in the tray when the window is closed</div>
      <div class="hint">Downloads and shortcuts keep working. Turn this off to quit instead.</div>
    </div>
    <Switch checked={closeToTray} label="Keep running in the tray when the window is closed" onchange={(v) => app.setCloseToTray(v)} />
  </div>
</section>

<DownloaderSettings />

<Shortcuts />

<style>
  h1 { margin-bottom: 20px; }
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .segmented { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-s); padding: 2px; }
  .segmented button { border: 0; background: transparent; color: var(--text-2); padding: 5px 12px; border-radius: 6px; cursor: pointer; transition: background var(--dur-1), color var(--dur-1); }
  .segmented button.on { background: var(--surface-3); color: var(--text); }
</style>
