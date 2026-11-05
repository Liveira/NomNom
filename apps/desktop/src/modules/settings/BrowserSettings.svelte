<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { browserIpc, type BrowserStatus } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';

  let status = $state<BrowserStatus | null>(null);
  let message = $state('');
  let busy = $state(false);

  async function refresh() {
    status = await browserIpc.status();
  }
  void refresh();

  async function act(action: () => Promise<void>, done: string) {
    busy = true;
    message = '';
    try {
      await action();
      message = done;
    } catch (e) {
      message = String(e);
    } finally {
      busy = false;
      await refresh();
    }
  }

  const launch = $derived(app.settings?.browser.launch_from_browser ?? true);
  const connected = $derived(status?.state === 'connected');
  const label = $derived(
    status?.state === 'connected' ? 'Connected' : status?.state === 'needs_repair' ? 'Needs repair' : 'Not connected',
  );
</script>

<section class="group" aria-labelledby="fx">
  <h3 id="fx">Firefox</h3>
  <p class="intro">
    The NomNom extension adds a download button to videos and posts. It works by sending links to
    this app, so NomNom needs to be connected to Firefox.
  </p>

  <div class="row">
    <div>
      <div class="label">
        <span class="dot" class:on={connected} class:warn={status?.state === 'needs_repair'}></span>{label}
      </div>
      {#if status?.detail}<div class="hint">{status.detail}</div>{/if}
      {#if status && !status.helper_found}
        <div class="hint bad">The helper program (nomnom-native-host.exe) is missing. Reinstall NomNom.</div>
      {/if}
    </div>
    <div class="buttons">
      {#if status?.state === 'connected'}
        <button class="plain" disabled={busy} onclick={() => act(browserIpc.disconnect, 'Disconnected.')}>Disconnect</button>
      {:else}
        <button class="plain primary" disabled={busy || !status?.helper_found} onclick={() => act(browserIpc.connect, 'Connected. Install the extension in Firefox to finish.')}>
          {status?.state === 'needs_repair' ? 'Repair connection' : 'Connect Firefox'}
        </button>
      {/if}
    </div>
  </div>

  <div class="row">
    <div>
      <div class="label">Start NomNom in the background when Firefox sends a download</div>
      <div class="hint">NomNom starts without opening a window. Turn this off if you only want downloads to work while NomNom is already running.</div>
    </div>
    <Switch checked={launch} label="Start NomNom in the background when Firefox sends a download" onchange={(v) => app.update({ launchFromBrowser: v })} />
  </div>

  <h3 class="sub">The extension</h3>
  <p class="hint">
    NomNom for Firefox is a separate add-on. To try it now, open <span class="mono">about:debugging</span> in Firefox,
    choose This Firefox, then Load Temporary Add-on, and pick <span class="mono">manifest.json</span> from the
    extension's <span class="mono">dist</span> folder. A temporary add-on is removed when Firefox restarts. A permanent
    install needs a signed package.
  </p>
  {#if message}<p class="hint" role="status">{message}</p>{/if}
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 8px; }
  h3.sub { margin-top: 20px; margin-bottom: 8px; }
  .intro { margin: 0 0 14px; color: var(--text-2); }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .row + .row { margin-top: 14px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .bad { color: var(--danger); }
  .mono { font-family: Consolas, monospace; background: var(--surface-2); padding: 0 4px; border-radius: 4px; user-select: text; }
  .dot { display: inline-block; width: 9px; height: 9px; border-radius: 50%; background: var(--text-3); margin-right: 8px; }
  .dot.on { background: var(--ok); }
  .dot.warn { background: var(--danger); }
  .buttons { display: flex; gap: 8px; flex: none; }
  .plain { padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); cursor: pointer; }
  .plain.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain.primary:hover:not(:disabled) { background: var(--accent-hover); }
  .plain:disabled { opacity: 0.5; cursor: default; }
</style>
