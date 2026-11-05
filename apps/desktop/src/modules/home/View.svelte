<script lang="ts">
  import CatNap from '../../lib/ui/CatNap.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { app } from '../../app/state.svelte';
  import { progressLine } from '../../lib/format';
  import { downloads } from '../downloads/store.svelte';

  const active = $derived(downloads.active.slice(0, 3));
  const more = $derived(Math.max(0, downloads.active.length - 3));
</script>

<h1>Home</h1>

<section class="card">
  <div class="head">
    <h3>Active downloads</h3>
    <button class="link" onclick={() => app.go('downloads')}>Open NomNom Downloader</button>
  </div>
  {#if active.length === 0}
    <div class="nap">
      <CatNap />
      <p class="empty">Nothing downloading. Paste a link in NomNom Downloader to wake the cat.</p>
    </div>
  {:else}
    <ul>
      {#each active as job (job.id)}
        {@const live = downloads.progressFor(job.id)}
        <li>
          <div class="line">
            <span class="title">{job.title}</span>
            <span class="state tabular">
              {#if job.status === 'paused'}Paused
              {:else if live}{progressLine(live)}
              {:else}Waiting{/if}
            </span>
          </div>
          {#if job.status === 'running'}<ProgressBar value={live?.fraction ?? null} label={`Progress for ${job.title}`} />{/if}
        </li>
      {/each}
    </ul>
    {#if more > 0}<p class="empty">and {more} more</p>{/if}
  {/if}
</section>

<style>
  h1 { margin-bottom: 20px; }
  .card { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; max-width: 640px; }
  .head { display: flex; justify-content: space-between; align-items: baseline; margin-bottom: 10px; }
  .empty { margin: 0; color: var(--text-3); }
  .nap { display: grid; justify-items: center; gap: 8px; padding: 6px 0 2px; text-align: center; }
  ul { list-style: none; margin: 0 0 8px; padding: 0; display: grid; gap: 12px; }
  .line { display: flex; justify-content: space-between; gap: 12px; margin-bottom: 6px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .state { color: var(--text-2); flex: none; }
  .link { border: 0; background: none; color: var(--accent-text); cursor: pointer; padding: 0; }
</style>
