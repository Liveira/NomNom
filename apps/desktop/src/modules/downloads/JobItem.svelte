<script lang="ts">
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { formatBytes, formatEta, formatSpeed } from '../../lib/format';
  import { ipc, type JobRow } from '../../lib/ipc';
  import { downloads } from './store.svelte';

  let { job }: { job: JobRow } = $props();

  const live = $derived(downloads.progressFor(job.id));
  const label = $derived(`${job.source}${job.format && job.format !== 'original' ? ' · ' + job.format.toUpperCase() : ''}${job.quality && job.quality !== 'best' ? ' · ' + job.quality + 'p' : ''}`);
  let copied = $state(false);
  let problem = $state('');

  async function run(action: () => Promise<unknown>) {
    problem = '';
    try {
      await action();
    } catch (e) {
      problem = String(e);
    }
    downloads.refreshSoon();
  }

  async function copyUrl() {
    await navigator.clipboard.writeText(job.url);
    copied = true;
    setTimeout(() => (copied = false), 1500);
  }
</script>

<div class="item" class:done={job.status === 'done'}>
  <div class="top">
    <div class="name" title={job.title}>{job.title}</div>
    <div class="actions">
      {#if job.status === 'running' || job.status === 'queued'}
        <button onclick={() => run(() => ipc.pause(job.id))} disabled={job.status === 'queued'}>Pause</button>
        <button onclick={() => run(() => ipc.cancel(job.id))}>Cancel</button>
      {:else if job.status === 'paused'}
        <button onclick={() => run(() => ipc.resume(job.id))}>Resume</button>
        <button onclick={() => run(() => ipc.cancel(job.id))}>Cancel</button>
      {:else if job.status === 'done'}
        <button onclick={() => run(() => ipc.openDownload(job.id, false))}>Open</button>
        <button onclick={() => run(() => ipc.openDownload(job.id, true))}>Open folder</button>
        <button onclick={copyUrl}>{copied ? 'Copied' : 'Copy link'}</button>
        <button onclick={() => run(() => ipc.remove(job.id))} aria-label="Remove from history">Remove</button>
      {:else}
        <button onclick={() => run(() => ipc.retry(job.id))}>Try again</button>
        <button onclick={copyUrl}>{copied ? 'Copied' : 'Copy link'}</button>
        <button onclick={() => run(() => ipc.remove(job.id))} aria-label="Remove from history">Remove</button>
      {/if}
    </div>
  </div>

  {#if job.status === 'running'}
    <ProgressBar value={live?.fraction ?? null} label={`Progress for ${job.title}`} />
    <div class="sub tabular">
      <span>{label}</span>
      <span>
        {live?.fraction != null ? `${Math.round(live.fraction * 100)}%` : 'Starting'}
        {#if formatSpeed(live?.speedBps)} · {formatSpeed(live?.speedBps)}{/if}
        {#if live?.etaReliable} · {formatEta(live?.etaSecs)}{/if}
      </span>
    </div>
  {:else}
    <div class="sub">
      <span>{label}</span>
      <span class:bad={job.status === 'failed'}>
        {#if job.status === 'queued'}Waiting
        {:else if job.status === 'paused'}Paused
        {:else if job.status === 'cancelled'}Cancelled
        {:else if job.status === 'failed'}{job.error ?? "Couldn't download this video."}
        {:else}Done{job.bytes ? ` · ${formatBytes(job.bytes)}` : ''}{/if}
      </span>
    </div>
  {/if}
  {#if problem}<div class="sub bad">{problem}</div>{/if}
</div>

<style>
  .item { display: grid; gap: 8px; padding: 10px 12px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); animation: in var(--dur-2) var(--ease-spring); }
  @keyframes in { from { opacity: 0; transform: translateY(4px) scale(0.99); } to { opacity: 1; transform: none; } }
  .top { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .name { font-weight: 500; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; min-width: 0; }
  .actions { display: flex; gap: 6px; flex: none; }
  button { padding: 3px 10px; font-size: 12.5px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  .sub { display: flex; justify-content: space-between; gap: 12px; color: var(--text-2); font-size: 12.5px; }
  .bad { color: var(--danger); }
</style>
