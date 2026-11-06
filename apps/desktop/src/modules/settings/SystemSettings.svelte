<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { integrationIpc, type IntegrationStatus } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { autostartLine, menuLine, needsRepair } from './integration';

  let status = $state<IntegrationStatus | null>(null);
  let busy = $state(false);
  let error = $state('');

  async function refresh() {
    status = await integrationIpc.status();
  }
  void refresh();

  const menuOn = $derived(app.settings?.system.shell_menu ?? true);

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
  <h3>Windows</h3>

  <div class="row">
    <div>
      <div class="label">Show NomNom in the right-click menu</div>
      <div class="hint">{status ? menuLine(status) : 'Checking…'}</div>
    </div>
    <Switch checked={menuOn} label="Show NomNom in the right-click menu" onchange={(v) => run(() => app.update({ shellMenu: v }))} />
  </div>

  <div class="row">
    <div>
      <div class="label">Start NomNom with Windows</div>
      <div class="hint">{status ? autostartLine(status.autostart) : 'Checking…'}</div>
    </div>
    <Switch
      checked={status?.autostart ?? false}
      label="Start NomNom with Windows"
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
  .plain { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain:disabled { opacity: 0.5; cursor: default; }
</style>
