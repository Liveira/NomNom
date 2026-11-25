<script lang="ts">
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import { ipc, onUpdateProgress, updateIpc } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { installLine, lastCheckedLine, percent } from './update';

  let version = $state('');
  let checking = $state(false);
  let installing = $state(false);
  let note = $state('');
  let error = $state('');
  let done = $state(0);
  let total = $state<number | null>(null);

  void updateIpc.version().then((v) => (version = v));
  let stop: (() => void) | null = null;
  void onUpdateProgress((p) => {
    done = p.done;
    total = p.total;
  }).then((u) => (stop = u));
  onDestroy(() => stop?.());

  const found = $derived(app.newVersion);
  const auto = $derived(app.settings?.updates.auto_check ?? true);
  const pct = $derived(percent(done, total));

  async function check() {
    checking = true;
    error = '';
    note = '';
    try {
      const r = await updateIpc.check();
      if (r.state === 'available') app.newVersion = r;
      else {
        app.newVersion = null;
        note = `NomNom ${r.current} is the latest version.`;
      }
      app.settings = await ipc.getSettings();
    } catch (e) {
      error = String(e);
    } finally {
      checking = false;
    }
  }

  async function install() {
    installing = true;
    error = '';
    note = '';
    done = 0;
    total = null;
    try {
      const msg = await updateIpc.install();
      if (msg) {
        note = msg;
        app.newVersion = null;
      }
    } catch (e) {
      error = String(e);
    } finally {
      installing = false;
    }
  }
</script>

<section class="group">
  <h3>Updates</h3>

  <div class="row">
    <div>
      <div class="label">NomNom {version}</div>
      <div class="hint">{lastCheckedLine(app.settings?.updates.last_checked ?? 0)}</div>
    </div>
    <div class="row-actions">
      <button class="plain" onclick={() => app.go('changelog')}>Patch notes</button>
      <button class="plain" disabled={checking || installing} onclick={check}>{checking ? 'Checking…' : 'Check for updates'}</button>
    </div>
  </div>

  <div class="row">
    <div>
      <div class="label">Check for updates automatically</div>
      <div class="hint">About once a day, NomNom reads the latest release page on GitHub. It sends nothing about you, and installs nothing on its own.</div>
    </div>
    <Switch checked={auto} label="Check for updates automatically" onchange={(v) => app.update({ autoUpdateCheck: v })} />
  </div>

  {#if found}
    <div class="found" role="status">
      <div class="label">Version {found.version} is available</div>
      {#if found.notes}<pre class="notes">{found.notes}</pre>{/if}
      <div class="hint">{installLine(found)}</div>
      {#if installing}
        <ProgressBar value={pct == null ? null : pct / 100} label="Downloading the update" />
      {:else}
        <div class="actions">
          <button class="primary" onclick={install}>Download and install</button>
          <a class="link" href={found.page} target="_blank" rel="noreferrer">Release page</a>
        </div>
      {/if}
    </div>
  {:else if note}
    <p class="hint ok" role="status">{note}</p>
  {/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .ok { margin: 8px 0 0; }
  .error { color: var(--danger); margin: 8px 0 0; }
  .found { margin-top: 8px; padding: 12px; border: 1px solid var(--border-strong); border-radius: var(--radius-s); background: var(--surface-2); display: grid; gap: 8px; }
  .notes { margin: 0; max-height: 160px; overflow: auto; white-space: pre-wrap; font: inherit; font-size: 13px; color: var(--text-2); }
  .actions { display: flex; gap: 14px; align-items: center; }
  .row-actions { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
  .plain, .primary { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  .primary { background: var(--accent); border-color: var(--accent); color: var(--on-accent, #fff); font-weight: 600; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain:disabled { opacity: 0.5; cursor: default; }
  .link { color: var(--accent-text); }
</style>
