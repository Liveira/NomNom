<script lang="ts">
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { noticeIpc, onNoticesChanged, type NoticeItem } from '../../lib/ipc';
  import { dayLabel, groupByDay } from './logic';

  let items = $state<NoticeItem[]>([]);
  let error = $state('');
  const days = $derived(groupByDay(items, Math.floor(Date.now() / 1000)));

  async function load() {
    try {
      items = await noticeIpc.list();
    } catch (e) {
      error = String(e);
    }
  }
  void load();
  let stop: (() => void) | null = null;
  void onNoticesChanged(() => void load()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  const icon = { info: 'info', success: 'ok', warning: 'warn', error: 'error' } as const;
  const time = (at: number) => new Date(at * 1000).toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });

  async function clear() {
    await noticeIpc.clear().catch((e) => (error = String(e)));
    await load();
  }
  async function remove(i: NoticeItem) {
    await noticeIpc.remove(i.id).catch((e) => (error = String(e)));
    await load();
  }
</script>

<header class="head">
  <h1><Icon name="bell" size={24} /> Notifications</h1>
  <p class="sub">What NomNom told you in the last seven days. They stay on this computer only.</p>
  {#if items.length > 0}<button class="plain" onclick={() => void clear()}>Clear all</button>{/if}
</header>
{#if error}<p class="bad" role="alert">{error}</p>{/if}

{#if items.length === 0}
  <p class="hint">Nothing yet. When NomNom finishes a download, reaches a goal or reminds you of something, it shows up here too.</p>
{:else}
  {#each days as d (d.key)}
    <h2>{dayLabel(d.key, d.daysAgo)}</h2>
    <ul class="list">
      {#each d.items as n (n.id)}
        <li class={n.kind}>
          <span class="ico"><Icon name={icon[n.kind] ?? 'info'} size={17} /></span>
          <div class="body">
            <strong translate="no">{n.title}</strong>
            {#if n.body}<span class="text" translate="no">{n.body}</span>{/if}
          </div>
          <span class="time tabular">{time(n.at)}</span>
          <div class="btns">
            {#if n.action}<button class="plain small" onclick={() => app.go(n.action!)}>Open</button>{/if}
            <button class="plain small" aria-label="Remove this notice" onclick={() => void remove(n)}><Icon name="x" size={14} /></button>
          </div>
        </li>
      {/each}
    </ul>
  {/each}
{/if}

<style>
  .head { display: grid; gap: 8px; margin-bottom: 8px; max-width: 760px; justify-items: start; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  h2 { font-size: 14px; margin: 20px 0 8px; color: var(--text-2); }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; max-width: 620px; }
  .bad { color: var(--danger); font-size: 13px; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; max-width: 860px; }
  li { display: flex; align-items: center; gap: 12px; padding: 9px 12px; border-radius: var(--radius-s); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); }
  .ico { flex: none; width: 32px; height: 32px; border-radius: 10px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); }
  li.success .ico { color: var(--ok); }
  li.warning .ico, li.error .ico { color: var(--danger); }
  .body { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .text { color: var(--text-2); font-size: 13px; overflow-wrap: anywhere; }
  .time { color: var(--text-3); font-size: 12px; }
  .btns { display: flex; gap: 6px; }
  button.plain { display: inline-flex; align-items: center; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.small { padding: 4px 10px; font-size: 12.5px; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
