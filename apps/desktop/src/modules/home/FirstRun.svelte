<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { browserIpc } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';

  let menu = $state(true);
  let firefox = $state(true);
  let startup = $state(false);
  let busy = $state(false);
  let error = $state('');

  async function finish() {
    busy = true;
    error = '';
    try {
      if (firefox) await browserIpc.connect();
    } catch (e) {
      // Firefox not being found shouldn't stop the rest of the setup.
      error = `Firefox wasn't connected: ${e}`;
    }
    try {
      await app.update({ shellMenu: menu, autostart: startup, firstRunDone: true });
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function skip() {
    busy = true;
    try {
      // Nothing is added to Windows; the choices stay available in Settings.
      await app.update({ shellMenu: false, autostart: false, firstRunDone: true });
    } finally {
      busy = false;
    }
  }
</script>

<section class="card" aria-label="First-time setup">
  <h3>Set up NomNom</h3>
  <p class="intro">Pick what NomNom may add to Windows. You can change any of this later in Settings.</p>

  <div class="row">
    <div>
      <div class="label">Add NomNom to the right-click menu</div>
      <div class="hint">Convert pictures, sound, video and documents from Explorer.</div>
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
      <div class="label">Start NomNom with Windows</div>
      <div class="hint">Runs in the tray, without a window.</div>
    </div>
    <Switch checked={startup} label="Start NomNom with Windows" onchange={(v) => (startup = v)} />
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="actions">
    <button class="plain" disabled={busy} onclick={skip}>Not now</button>
    <button class="primary" disabled={busy} onclick={finish}>Finish setup</button>
  </div>
</section>

<style>
  .card { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; max-width: 640px; margin-bottom: 14px; }
  h3 { margin-bottom: 4px; }
  .intro { margin: 0 0 8px; color: var(--text-2); }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .error { color: var(--danger); margin: 8px 0 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 10px; }
  button { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
</style>
