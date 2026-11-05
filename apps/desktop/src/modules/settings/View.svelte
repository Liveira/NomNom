<script lang="ts">
  import { app } from '../../app/state.svelte';
  import type { Theme } from '../../lib/ipc';

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
    <div>
      <div class="label">Theme</div>
    </div>
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
    <button
      class="switch"
      role="switch"
      aria-checked={closeToTray}
      aria-label="Keep running in the tray when the window is closed"
      class:on={closeToTray}
      onclick={() => app.setCloseToTray(!closeToTray)}><span></span></button
    >
  </div>
</section>

<style>
  h1 { margin-bottom: 20px; }
  .group {
    max-width: 640px;
    margin-bottom: 16px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 16px 18px;
  }
  h3 { margin-bottom: 12px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }

  .segmented { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-s); padding: 2px; }
  .segmented button {
    border: 0; background: transparent; color: var(--text-2); padding: 5px 12px;
    border-radius: 5px; cursor: pointer; transition: background var(--dur-1), color var(--dur-1);
  }
  .segmented button.on { background: var(--surface-3); color: var(--text); }

  .switch {
    flex: none; width: 38px; height: 22px; border-radius: 11px; border: 0; padding: 2px;
    background: var(--surface-3); cursor: pointer; transition: background var(--dur-2);
  }
  .switch span {
    display: block; width: 18px; height: 18px; border-radius: 50%; background: var(--text-2);
    transition: transform var(--dur-2) var(--ease-spring), background var(--dur-2);
  }
  .switch.on { background: var(--accent); }
  .switch.on span { transform: translateX(16px); background: var(--on-accent); }
</style>
