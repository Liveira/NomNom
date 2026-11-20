<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { launcherIpc, recorderIpc, shortcutsIpc, type RecorderStatus, type ShortcutInfo } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import { every } from '../../lib/poll';
  import Switch from '../../lib/ui/Switch.svelte';

  let status = $state<RecorderStatus>({ recording: false, seconds: 0, clock: '00:00', file: null });
  let gif = $state(false);
  let key = $state<ShortcutInfo | null>(null);
  let error = $state('');
  let saved = $state('');

  async function look() {
    status = await recorderIpc.status().catch(() => status);
  }
  void look();
  const timer = every(1000, () => void look());
  onDestroy(timer);
  onMount(async () => {
    key = (await shortcutsIpc.status()).find((s) => s.id === 'record') ?? null;
  });

  async function run(job: () => Promise<unknown>) {
    error = '';
    saved = '';
    try {
      await job();
    } catch (e) {
      error = String(e);
    }
    await look();
  }
  async function stop() {
    error = '';
    try {
      saved = await recorderIpc.stop();
    } catch (e) {
      error = String(e);
    }
    await look();
  }
  async function openFolder() {
    error = '';
    try {
      await launcherIpc.openPath(await recorderIpc.folder(), false);
    } catch {
      error = 'There are no recordings yet.';
    }
  }
</script>

<header class="head">
  <h1><Icon name="film" size={24} /> Screen recorder</h1>
  <p class="sub">Record the screen, or part of it, as an MP4 video or a small GIF. There is no sound. It uses the ffmpeg that NomNom already sets up for File Tools.</p>
</header>

{#if isLinux}
  <p class="hint">Recording the screen works on Windows for now.</p>
{:else}
  <section class="card">
    {#if status.recording}
      <div class="live" role="timer"><span class="dot" aria-hidden="true"></span><strong class="tabular">{status.clock}</strong><span>Recording</span></div>
      <button class="primary" onclick={() => void stop()}>Stop and save</button>
    {:else}
      <div class="row">
        <button class="primary" onclick={() => void run(() => recorderIpc.startArea(gif))}>Record an area…</button>
        <button class="plain" onclick={() => void run(() => recorderIpc.startScreen(gif))}>Record the whole screen</button>
      </div>
      <div class="inline"><Switch checked={gif} label="Make a GIF instead of a video" onchange={(v) => (gif = v)} /><span>Make a GIF instead of a video (small, no sound, best for short clips)</span></div>
      <p class="hint">"Record an area" freezes the screen for a moment so you can drag over the part you want. {#if key}The shortcut <kbd>{key.accelerator}</kbd> does the same, and stops the recording.{/if}</p>
      {#if key?.error}<p class="bad" role="alert">{key.error}</p>{/if}
    {/if}
    <p class="hint">A recording stops by itself after 10 minutes. The toast that says it started is not part of the video if you record away from the corner.</p>
    {#if saved}<p class="ok" role="status">Saved: <span translate="no">{saved}</span></p>{/if}
    {#if error}<p class="bad" role="alert">{error}</p>{/if}
    <div class="row"><button class="plain" onclick={() => void openFolder()}>Open the recordings folder</button></div>
  </section>
{/if}

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; max-width: 620px; }
  .bad { color: var(--danger); font-size: 13px; margin: 0; }
  .ok { color: var(--ok); font-size: 13px; margin: 0; overflow-wrap: anywhere; }
  .card { display: grid; gap: 14px; justify-items: start; max-width: 680px; padding: 18px 20px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border-strong); }
  .row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .inline { display: flex; gap: 10px; align-items: center; font-size: 13.5px; }
  .live { display: flex; align-items: center; gap: 12px; font-size: 28px; }
  .live span:last-child { font-size: 14px; color: var(--text-3); }
  .dot { width: 14px; height: 14px; border-radius: 50%; background: var(--danger); animation: pulse 1.2s ease-in-out infinite; }
  @keyframes pulse { 50% { opacity: 0.35; } }
  @media (prefers-reduced-motion: reduce) { .dot { animation: none; } }
  kbd { font: 600 12px var(--font-mono); padding: 2px 8px; border-radius: 6px; border: 1px solid var(--border-strong); color: var(--text); }
  button.plain, button.primary { display: inline-flex; align-items: center; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
