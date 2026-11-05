<script lang="ts">
  import { formatBytes } from '../../lib/format';
  import { downloads } from './store.svelte';

  let {
    tool,
    text,
    compact = false,
  }: { tool: 'ytdlp' | 'ffmpeg'; text: string; compact?: boolean } = $props();

  const busy = $derived(downloads.installing?.tool === tool);
  const progress = $derived(downloads.installing);
</script>

<div class="banner" class:compact role="status">
  <span>{text}</span>
  {#if busy && progress}
    <span class="tabular">
      {formatBytes(progress.done)}{progress.total ? ` of ${formatBytes(progress.total)}` : ''}
    </span>
  {:else}
    <button onclick={() => downloads.install(tool)} disabled={downloads.installing != null}>
      Get {tool === 'ytdlp' ? 'yt-dlp' : 'FFmpeg'}
    </button>
  {/if}
  {#if downloads.installError}<span class="err">{downloads.installError}</span>{/if}
</div>

<style>
  .banner {
    display: flex; align-items: center; gap: 12px; flex-wrap: wrap; max-width: 760px;
    margin: 0 0 14px; padding: 10px 12px; border-radius: var(--radius-s);
    background: var(--surface-2); border: 1px solid var(--border-strong);
  }
  .compact { margin: 4px 0 0; font-size: 12.5px; }
  button {
    padding: 5px 12px; border-radius: var(--radius-s); border: 0; cursor: pointer;
    background: var(--accent); color: var(--on-accent); font-weight: 600;
  }
  button:disabled { opacity: 0.5; cursor: default; }
  .err { color: var(--danger); flex-basis: 100%; }
</style>
