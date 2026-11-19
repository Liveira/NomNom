<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { integrationIpc, settingsFileIpc, type IntegrationStatus } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import Switch from '../../lib/ui/Switch.svelte';
  import { autostartLine, menuLine, needsRepair } from './integration';

  let status = $state<IntegrationStatus | null>(null);
  let busy = $state(false);
  let error = $state('');

  async function refresh() {
    status = await integrationIpc.status();
  }
  void refresh();

  let fileNote = $state('');
  async function exportSettings() {
    fileNote = '';
    error = '';
    const path = await save({ defaultPath: 'nomnom-settings.json', filters: [{ name: 'NomNom settings', extensions: ['json'] }] });
    if (!path) return;
    try {
      await settingsFileIpc.save(path);
      fileNote = 'Saved.';
    } catch (e) {
      error = String(e);
    }
  }
  async function importSettings() {
    fileNote = '';
    error = '';
    const path = await open({ multiple: false, filters: [{ name: 'NomNom settings', extensions: ['json'] }] });
    if (typeof path !== 'string') return;
    try {
      app.settings = await settingsFileIpc.load(path);
      fileNote = 'Loaded. Some changes show after you restart NomNom.';
    } catch (e) {
      error = String(e);
    }
  }

  const menuOn = $derived(app.settings?.system.shell_menu ?? false);

  async function run(action: () => Promise<unknown>) {
    busy = true;
    error = '';
    try {
      await action();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      await refresh();
    }
  }
</script>

<section class="group">
  <h3>{(status?.platform ?? (isLinux ? 'linux' : 'windows')) === 'linux' ? 'Linux desktop' : 'Windows'}</h3>

  <div class="row">
    <div>
      <div class="label">Show NomNom in the right-click menu</div>
      <div class="hint">{status ? menuLine(status) : 'Checking…'}</div>
    </div>
    <Switch checked={menuOn} label="Show NomNom in the right-click menu" onchange={(v) => run(() => app.update({ shellMenu: v }))} />
  </div>

  <div class="row">
    <div>
      <div class="label">{(status?.platform ?? (isLinux ? 'linux' : 'windows')) === 'linux' ? 'Start NomNom on login' : 'Start NomNom with Windows'}</div>
      <div class="hint">{status ? autostartLine(status.autostart, status.platform ?? (isLinux ? 'linux' : 'windows')) : 'Checking…'}</div>
    </div>
    <Switch
      checked={status?.autostart ?? false}
      label={(status?.platform ?? (isLinux ? 'linux' : 'windows')) === 'linux' ? 'Start NomNom on login' : 'Start NomNom with Windows'}
      onchange={(v) => run(() => app.update({ autostart: v }))}
    />
  </div>

  {#if status && (needsRepair(status) || !status.has_modern_package)}
    <div class="row">
      <div class="hint">
        {#if needsRepair(status)}The menu isn't where it should be.{:else}This copy of NomNom uses the classic menu.{/if}
      </div>
      <button class="plain" disabled={busy} onclick={() => run(() => integrationIpc.repair())}>Repair</button>
    </div>
  {/if}
  <div class="row">
    <div>
      <div class="label">Your settings</div>
      <div class="hint">Save them to a file to keep a copy or move to another computer. How NomNom starts on each computer is not carried over.</div>
      {#if fileNote}<div class="hint" role="status">{fileNote}</div>{/if}
    </div>
    <div class="pair">
      <button class="plain" onclick={() => void exportSettings()}>Save to a file…</button>
      <button class="plain" onclick={() => void importSettings()}>Load from a file…</button>
    </div>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .error { color: var(--danger); margin: 8px 0 0; }
  .pair { display: flex; gap: 8px; flex-wrap: wrap; justify-content: flex-end; }
  .plain { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain:disabled { opacity: 0.5; cursor: default; }
</style>
