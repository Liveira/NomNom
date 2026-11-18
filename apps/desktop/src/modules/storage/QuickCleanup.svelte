<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import { formatBytes } from '../../lib/format';
  import { cleanupIpc, type StCleanSpot } from '../../lib/ipc';
  import { storage } from './store.svelte';

  let spots = $state<StCleanSpot[] | null>(null);
  let looking = $state(false);
  let error = $state('');

  const total = $derived((spots ?? []).reduce((n, s) => n + s.size, 0));

  async function look() {
    looking = true;
    error = '';
    try {
      spots = await cleanupIpc.spots();
    } catch (e) {
      error = String(e);
    } finally {
      looking = false;
    }
  }
</script>

<section class="cleanup" aria-labelledby="qc">
  <div class="head">
    <div>
      <h2 id="qc"><Icon name="wand" size={18} /> Quick cleanup</h2>
      <p class="hint">
        Places where programs keep files they can make again, like browser caches and temporary files. NomNom only measures them. Nothing is deleted until you review the list and confirm.
      </p>
    </div>
    <button class="primary" disabled={looking} onclick={look}>
      {looking ? 'Measuring…' : spots ? 'Measure again' : 'Look for things to clean'}
    </button>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if spots}
    {#if spots.length === 0}
      <p class="empty" role="status">Nothing worth cleaning was found.</p>
    {:else}
      <p class="sum tabular" role="status">{formatBytes(total)} could be cleaned in {spots.length} {spots.length === 1 ? 'place' : 'places'}.</p>
      <ul class="spots">
        {#each spots as s, i (s.id + s.path)}
          <li style:--i={i}>
            <div class="what">
              <strong>{s.title}</strong>
              <span class="why">{s.why}</span>
              <span class="path" translate="no" title={s.path}>{s.path}</span>
            </div>
            <div class="size tabular">
              <strong>{s.partial ? '≥ ' : ''}{formatBytes(s.size)}</strong>
              <span>{s.files.toLocaleString()} files</span>
            </div>
            <button class="plain" onclick={() => storage.cleanFolder(s.path)}>Review and clean…</button>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .cleanup { margin-top: 22px; padding: 18px 20px; border-radius: var(--radius-l); background: color-mix(in srgb, var(--surface-1) 88%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-soft); }
  .head { display: flex; justify-content: space-between; gap: 18px; align-items: flex-start; flex-wrap: wrap; }
  h2 { font-size: 16px; display: flex; align-items: center; gap: 8px; color: var(--accent-text); }
  .hint { margin: 6px 0 0; max-width: 62ch; color: var(--text-2); font-size: 13px; }
  .sum { margin: 14px 0 6px; color: var(--text-2); font-weight: 700; }
  .empty { color: var(--text-3); margin: 14px 0 0; }
  .error { color: var(--danger); margin: 10px 0 0; }
  .spots { list-style: none; margin: 8px 0 0; padding: 0; display: grid; gap: 8px; }
  li { display: grid; grid-template-columns: minmax(0, 1fr) auto auto; gap: 16px; align-items: center; padding: 12px 14px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border); animation: rise var(--dur-4) var(--ease-out) backwards; animation-delay: calc(var(--i) * 50ms); }
  .what { display: grid; gap: 2px; min-width: 0; }
  .why { color: var(--text-2); font-size: 12.5px; }
  .path { color: var(--text-3); font: 12px var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .size { display: grid; text-align: right; }
  .size span { color: var(--text-3); font-size: 12px; }
  button { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-3); color: inherit; cursor: pointer; white-space: nowrap; }
  button:hover:not(:disabled) { border-color: var(--border-glow); }
  button:disabled { opacity: 0.55; cursor: default; }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  @media (max-width: 760px) { li { grid-template-columns: 1fr; } .size { text-align: left; } }
</style>
