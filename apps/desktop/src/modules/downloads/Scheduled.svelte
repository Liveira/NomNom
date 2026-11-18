<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { onScheduledChanged, scheduleIpc, type Scheduled } from '../../lib/ipc';
  import { whenText } from './when';

  let list = $state<Scheduled[]>([]);
  async function refresh() {
    list = await scheduleIpc.list();
  }
  void refresh();
  let stop: (() => void) | null = null;
  void onScheduledChanged(() => void refresh()).then((u) => (stop = u));
  const poll = setInterval(() => void refresh(), 20000);
  onDestroy(() => {
    stop?.();
    clearInterval(poll);
  });

  /** The parent calls this after it adds one. */
  export function reload() {
    void refresh();
  }

  async function cancel(id: string) {
    await scheduleIpc.cancel(id);
    await refresh();
  }
</script>

{#if list.length > 0}
  <section class="sched" aria-label="Scheduled downloads">
    <h2>Scheduled</h2>
    <ul>
      {#each list as s (s.id)}
        <li>
          <span class="ico"><Icon name="clock" size={16} /></span>
          <span class="what"><strong translate="no" title={s.url}>{s.title ?? s.url}</strong><span class="when">Starts {whenText(s.run_at)}</span></span>
          <button class="x" aria-label="Cancel this scheduled download" onclick={() => cancel(s.id)}><Icon name="x" size={14} /></button>
        </li>
      {/each}
    </ul>
    <p class="hint">NomNom has to be running (the tray is fine) for a scheduled download to start.</p>
  </section>
{/if}

<style>
  .sched { margin-bottom: 18px; }
  h2 { margin-bottom: 8px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  li { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 12px; align-items: center; padding: 10px 14px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border-strong); animation: rise var(--dur-3) var(--ease-out); }
  .ico { display: grid; place-items: center; width: 30px; height: 30px; border-radius: 10px; background: var(--surface-3); color: var(--accent-text); }
  .what { display: grid; min-width: 0; }
  .what strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .when { color: var(--text-3); font-size: 12.5px; }
  .x { display: grid; place-items: center; width: 28px; height: 28px; border: 1px solid var(--border); border-radius: 8px; background: var(--surface-2); color: inherit; cursor: pointer; }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 8px 0 0; }
</style>
