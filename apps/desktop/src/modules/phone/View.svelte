<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { openPath } from '@tauri-apps/plugin-opener';
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import { onPhoneChanged, phoneIpc, type PhoneStatus } from '../../lib/ipc';
  import { IDLE_CHOICES, fileNameOf } from './logic';

  let status = $state<PhoneStatus | null>(null);
  let busy = $state(false);
  let error = $state('');
  let copied = $state(false);

  async function load() {
    status = await phoneIpc.status().catch(() => null);
  }
  void load();
  let stop: (() => void) | null = null;
  void onPhoneChanged(() => void load()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  async function start() {
    busy = true;
    error = '';
    try {
      status = await phoneIpc.start();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function halt() {
    status = await phoneIpc.stop();
  }
  async function add() {
    const picked = await open({ multiple: true });
    if (!picked) return;
    error = '';
    try {
      status = await phoneIpc.offer(Array.isArray(picked) ? picked : [picked]);
    } catch (e) {
      error = String(e);
    }
  }
  async function pickFolder() {
    const dir = await open({ directory: true });
    if (typeof dir === 'string') {
      await app.update({ phoneDir: dir });
      await load();
    }
  }
  async function copyLink() {
    if (!status?.url) return;
    await navigator.clipboard.writeText(status.url).catch(() => {});
    copied = true;
    setTimeout(() => (copied = false), 1800);
  }
  const idle = $derived(app.settings?.phone.idle_minutes ?? 15);
</script>

<header class="head">
  <h1><Icon name="smartphone" size={24} /> Phone transfer</h1>
  <p class="sub">Send photos and files from your phone to this computer, or the other way, over your own Wi-Fi. Nothing goes through the internet.</p>
</header>

{#if status && !status.running}
  <section class="card start">
    <div class="how">
      <ol>
        <li>Press Start and a QR code appears.</li>
        <li>Point your phone's camera at it. Your phone and this computer must be on the same Wi-Fi.</li>
        <li>On the phone, pick files to send here, or download what you offer from here.</li>
      </ol>
      <button class="primary" onclick={start} disabled={busy}>{busy ? 'Starting…' : 'Start sharing'}</button>
    </div>
    <div class="row">
      <div>
        <div class="label">Save files from the phone in</div>
        <div class="hint" translate="no">{status.dest}</div>
      </div>
      <button class="plain" onclick={pickFolder}>Change…</button>
    </div>
    <div class="row">
      <div>
        <div class="label">Turn sharing off by itself after</div>
        <div class="hint">When nobody uses it for this long.</div>
      </div>
      <select aria-label="Turn sharing off by itself after" value={idle} onchange={(e) => app.update({ phoneIdleMinutes: Number(e.currentTarget.value) })}>
        {#each IDLE_CHOICES as c (c.minutes)}<option value={c.minutes}>{c.label}</option>{/each}
      </select>
    </div>
    <p class="hint">Anyone on your Wi-Fi who has the link can use it while it is on, so use it only on a network you trust. The link is different each time. The first time, Windows may ask to let NomNom use the network: allow it for private networks.</p>
  </section>
{:else if status}
  <section class="card live">
    <div class="qrbox">
      {#if status.qr}<div class="qr" aria-label="QR code to open the page on your phone">{@html status.qr}</div>{/if}
      <div class="link">
        <div class="label">Or type this in your phone's browser</div>
        <code class="mono" translate="no">{status.url}</code>
        <div class="buttons">
          <button class="plain" onclick={copyLink}>{copied ? 'Copied' : 'Copy link'}</button>
          <button class="plain danger" onclick={halt}>Stop sharing</button>
        </div>
        <div class="hint">It turns itself off after {status.idle_minutes} quiet minutes.</div>
      </div>
    </div>
  </section>

  <div class="two">
    <section class="card">
      <h2>From your phone</h2>
      {#if status.received.length === 0}
        <p class="hint">Files you send from the phone show up here. They are saved in <span translate="no">{status.dest}</span>.</p>
      {:else}
        <ul class="files">
          {#each status.received as r (r.path)}
            <li>
              <span class="n" translate="no" title={r.path}>{r.name}</span>
              <span class="s tabular">{formatBytes(r.size)}</span>
              <button class="icon" aria-label="Show in folder" title="Open" onclick={() => openPath(r.path).catch(() => {})}><Icon name="folder" size={15} /></button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="card">
      <div class="listhead">
        <h2>To your phone</h2>
        <button class="plain" onclick={add}><Icon name="plus" size={15} /> Add files</button>
      </div>
      {#if status.offered.length === 0}
        <p class="hint">Add files here and they appear on your phone's page, ready to download.</p>
      {:else}
        <ul class="files">
          {#each status.offered as o (o.id)}
            <li>
              <span class="n" translate="no">{o.name}</span>
              <span class="s tabular">{formatBytes(o.size)}</span>
              <button class="icon" aria-label="Stop offering this file" title="Remove" onclick={async () => (status = await phoneIpc.unoffer(o.id))}><Icon name="x" size={15} /></button>
            </li>
          {/each}
        </ul>
      {/if}
    </section>
  </div>
{/if}

{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
  .head h1 { display: flex; align-items: center; gap: 10px; margin: 0 0 4px; }
  .sub { margin: 0 0 16px; color: var(--text-2); max-width: 640px; }
  .card { max-width: 860px; margin-bottom: 16px; padding: 16px 18px; border-radius: var(--radius-m); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-card); }
  h2 { margin: 0 0 8px; font-size: 16px; }
  ol { margin: 0 0 14px; padding-left: 20px; color: var(--text-2); display: grid; gap: 4px; }
  .how { padding-bottom: 12px; border-bottom: 1px solid var(--border); margin-bottom: 8px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .label { font-weight: 600; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .qrbox { display: flex; gap: 22px; align-items: center; flex-wrap: wrap; }
  .qr { width: 220px; height: 220px; border-radius: 14px; overflow: hidden; background: #fff; flex: none; }
  .qr :global(svg) { width: 100%; height: 100%; display: block; }
  .link { display: grid; gap: 8px; min-width: 0; flex: 1; }
  .mono { font-family: var(--font-mono); background: var(--surface-2); padding: 8px 10px; border-radius: var(--radius-xs); word-break: break-all; user-select: all; font-size: 13px; }
  .buttons { display: flex; gap: 8px; flex-wrap: wrap; }
  .two { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 16px; max-width: 860px; }
  .two .card { margin: 0; }
  .listhead { display: flex; justify-content: space-between; align-items: center; margin-bottom: 8px; }
  .listhead h2 { margin: 0; }
  .files { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; max-height: 280px; overflow: auto; }
  .files li { display: flex; align-items: center; gap: 10px; padding: 7px 10px; border-radius: var(--radius-xs); background: var(--surface-2); }
  .n { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .s { color: var(--text-3); font-size: 12px; flex: none; }
  select { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button.danger { color: var(--danger); border-color: var(--danger); }
  button:disabled { opacity: 0.5; cursor: default; }
  .icon { border: 0; background: none; color: var(--text-3); cursor: pointer; padding: 5px; border-radius: 8px; display: grid; }
  .icon:hover { background: var(--surface-3); color: var(--text); }
  .error { color: var(--danger); max-width: 860px; }
</style>
