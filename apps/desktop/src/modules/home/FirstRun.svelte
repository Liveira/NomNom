<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { browserIpc } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import Icon from '../../lib/icons/Icon.svelte';
  import { DEFAULT_PICKS, TOOLS, TOOL_SETS, activeToolSet, withTool } from '../../lib/tools';
  import Switch from '../../lib/ui/Switch.svelte';

  let picks = $state<string[]>([...DEFAULT_PICKS]);
  let menu = $state(true);
  let firefox = $state(true);
  let startup = $state(false);
  let busy = $state(false);
  let error = $state('');
  const currentSet = $derived(activeToolSet(picks));

  async function finish() {
    busy = true;
    error = '';
    if (firefox) {
      try {
        await browserIpc.connect();
      } catch (e) {
        error = `Firefox wasn't connected: ${e}. You can uncheck Connect Firefox or try again.`;
        busy = false;
        return;
      }
    }
    try {
      await app.update({ shellMenu: menu, autostart: startup, sidebarTools: picks, firstRunDone: true });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function skip() {
    busy = true;
    error = '';
    try {
      await app.update({
        shellMenu: false,
        autostart: false,
        sidebarTools: [...DEFAULT_PICKS],
        firstRunDone: true,
      });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card" aria-label="First-time setup">
  <h3>Set up NomNom</h3>
  <p class="intro">Pick what NomNom may add to {isLinux ? 'your desktop' : 'Windows'}. You can change any of this later in Settings.</p>

  <div class="picks">
    <div class="label">Choose a Tool Set or pick individual tools</div>
    <div class="hint">These go in the sidebar. The others stay on the Tools page, so nothing gets in your way.</div>
    <div class="sets-row">
      {#each TOOL_SETS as preset (preset.id)}
        <button
          type="button"
          class="set-btn"
          class:on={currentSet === preset.id}
          onclick={() => (picks = [...preset.tools])}
        >
          <Icon name={preset.icon} size={13} />
          <span>{preset.title}</span>
        </button>
      {/each}
    </div>
    <div class="chips">
      {#each TOOLS as t (t.id)}
        <label class="chip" class:on={picks.includes(t.id)}>
          <input type="checkbox" checked={picks.includes(t.id)} onchange={(e) => (picks = withTool(picks, t.id, e.currentTarget.checked))} />
          <Icon name={t.icon} size={15} />
          <span>{t.title}</span>
        </label>
      {/each}
    </div>
  </div>

  <div class="row">
    <div>
      <div class="label">Add NomNom to the right-click menu</div>
      <div class="hint">Convert files and analyze folders from {isLinux ? 'Dolphin, Files and Nemo' : 'Explorer'}.</div>
    </div>
    <Switch checked={menu} label="Add NomNom to the right-click menu" onchange={(v) => (menu = v)} />
  </div>
  <div class="row">
    <div>
      <div class="label">Connect Firefox</div>
      <div class="hint">Lets the NomNom extension send downloads to the app.</div>
    </div>
    <Switch checked={firefox} label="Connect Firefox" onchange={(v) => (firefox = v)} />
  </div>
  <div class="row">
    <div>
      <div class="label">{isLinux ? 'Start NomNom on login' : 'Start NomNom with Windows'}</div>
      <div class="hint">Runs in the tray, without a window, and tracks active Playtime hours quietly.</div>
    </div>
    <Switch checked={startup} label={isLinux ? 'Start NomNom on login' : 'Start NomNom with Windows'} onchange={(v) => (startup = v)} />
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="plain" disabled={busy} onclick={skip}>Not now</button>
    <button class="primary" disabled={busy} onclick={finish}>Finish setup</button>
  </div>
</section>

<style>
  .card {
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-m);
    padding: 18px 20px;
    margin-bottom: 18px;
    box-shadow: var(--shadow-card);
  }
  h3 { margin-bottom: 4px; font-size: 15px; }
  .intro { margin: 0 0 10px; color: var(--text-2); font-size: 13px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 9px 0; }
  .picks { padding: 6px 0 12px; border-bottom: 1px solid var(--border); }
  .sets-row { display: flex; flex-wrap: wrap; gap: 6px; margin-top: 9px; }
  .set-btn {
    display: inline-flex; align-items: center; gap: 6px; padding: 4px 10px;
    border-radius: var(--radius-xs); border: 1px solid var(--border); background: var(--surface-1);
    color: var(--text-2); font-size: 12px; font-weight: 600; cursor: pointer;
  }
  .set-btn:hover { background: var(--surface-2); color: var(--text); }
  .set-btn.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent-text); }
  .chips { display: flex; flex-wrap: wrap; gap: 7px; margin-top: 10px; }
  .chip { display: inline-flex; align-items: center; gap: 7px; padding: 5px 11px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); cursor: pointer; font-weight: 600; font-size: 12.5px; transition: background var(--dur-1), border-color var(--dur-1); }
  .chip.on { background: var(--accent-soft); border-color: var(--accent); color: var(--accent-text); }
  .chip input { position: absolute; opacity: 0; width: 0; height: 0; }
  .chip:focus-within { outline: 2px solid var(--focus); outline-offset: 2px; }
  .row + .row { border-top: 1px solid var(--border); }
  .label { font-weight: 600; font-size: 13.5px; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .error { color: var(--danger); margin: 8px 0 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 14px; }
  button { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; font-size: 13px; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
</style>
