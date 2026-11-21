<script lang="ts">
  import { onDestroy, onMount, tick } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { clipIpc, onClipboardChanged, type ClipEntry } from '../../lib/ipc';
  import { app } from '../../app/state.svelte';
  import { downloads } from '../downloads/store.svelte';
  import { jsonFormat } from '../toolbox/logic';
  import { ago, smartOf } from './logic';

  /** `popup`: the small window opened by the shortcut (Enter pastes). `page`: inside NomNom. */
  let { mode = 'page' }: { mode?: 'popup' | 'page' } = $props();

  let entries = $state<ClipEntry[]>([]);
  let query = $state('');
  let active = $state(0);
  let note = $state('');
  let search: HTMLInputElement | undefined = $state();
  let timer: ReturnType<typeof setTimeout> | undefined;

  async function load() {
    entries = await clipIpc.list(query, mode === 'popup' ? 60 : 300).catch(() => []);
    if (active >= entries.length) active = Math.max(0, entries.length - 1);
  }

  $effect(() => {
    void query;
    clearTimeout(timer);
    timer = setTimeout(() => void load(), 120);
    return () => clearTimeout(timer);
  });

  let stop: (() => void) | null = null;
  onMount(async () => {
    await load();
    await tick();
    search?.focus();
    if (mode === 'popup') await clipIpc.ready();
    stop = await onClipboardChanged(() => void load()).then((u) => u);
  });
  onDestroy(() => stop?.());

  async function use(e: ClipEntry) {
    try {
      await clipIpc.use(e.id, mode === 'popup');
      if (mode === 'page') {
        note = 'Copied. Paste it anywhere.';
        setTimeout(() => (note = ''), 2200);
      }
    } catch (err) {
      note = String(err);
    }
  }

  function say(text: string) {
    note = text;
    setTimeout(() => (note = ''), 2400);
  }
  async function download(url: string) {
    downloads.pendingUrl = url;
    app.go('downloads');
  }
  async function clean(url: string) {
    const c = await clipIpc.cleanLink(url).catch(() => null);
    if (!c) return say('There was no tracking in that link.');
    await navigator.clipboard.writeText(c).catch(() => {});
    say('Cleaned link copied.');
  }
  async function readable(e: ClipEntry) {
    const r = jsonFormat(await clipIpc.text(e.id).catch(() => ''));
    if (!r.ok) return say(r.error);
    await navigator.clipboard.writeText(r.value).catch(() => {});
    say('Readable JSON copied.');
  }

  async function pin(e: ClipEntry) {
    await clipIpc.pin(e.id, !e.pinned);
  }
  async function remove(e: ClipEntry) {
    await clipIpc.remove(e.id);
  }

  function onKey(ev: KeyboardEvent) {
    if (ev.key === 'ArrowDown') {
      ev.preventDefault();
      active = Math.min(entries.length - 1, active + 1);
      scrollTo();
    } else if (ev.key === 'ArrowUp') {
      ev.preventDefault();
      active = Math.max(0, active - 1);
      scrollTo();
    } else if (ev.key === 'Enter' && entries[active]) {
      ev.preventDefault();
      void use(entries[active]);
    } else if (ev.key === 'Escape' && mode === 'popup') {
      void clipIpc.close();
    } else if (ev.key === 'Delete' && ev.ctrlKey && entries[active]) {
      void remove(entries[active]);
    }
  }
  async function scrollTo() {
    await tick();
    document.querySelector('.clip-row.active')?.scrollIntoView({ block: 'nearest' });
  }
</script>

<div class="wrap" class:popup={mode === 'popup'}>
  <div class="searchbar">
    <Icon name="search" size={16} />
    <input
      bind:this={search}
      type="search"
      placeholder="Search what you copied"
      aria-label="Search what you copied"
      bind:value={query}
      onkeydown={onKey}
      spellcheck="false"
    />
  </div>

  {#if entries.length === 0}
    <p class="empty">{query ? 'Nothing matches that.' : 'Nothing here yet. Copy some text and it shows up.'}</p>
  {/if}

  <ul class="list" role="listbox" aria-label="Copied text">
    {#each entries as e, i (e.id)}
      {@const s = smartOf(e.preview, e.chars)}
      <li class="clip-row" class:active={i === active} class:pinned={e.pinned} role="option" aria-selected={i === active}>
        <button class="main" onclick={() => use(e)} onmousemove={() => (active = i)} title="Copy this again">
          <span class="text" translate="no">{#if s?.kind === 'colour'}<span class="sw" style="background:{s.hex}"></span>{/if}{e.preview}</span>
          <span class="meta tabular">{ago(e.last_at)}{e.copies > 1 ? ` · copied ${e.copies} times` : ''}{e.chars > 220 ? ` · ${e.chars} characters` : ''}</span>
        </button>
        {#if s && mode === 'page'}
          {#if s.kind === 'link'}
            <button class="act" onclick={() => download(s.url)}>Download</button>
            <button class="act" onclick={() => clean(s.url)}>Clean</button>
          {:else if s.kind === 'calc'}
            <span class="chip tabular" translate="no">= {s.answer}</span>
          {:else if s.kind === 'json'}
            <button class="act" onclick={() => readable(e)}>Make readable</button>
          {/if}
        {:else if s?.kind === 'calc'}
          <span class="chip tabular" translate="no">= {s.answer}</span>
        {/if}
        <button class="tool" class:on={e.pinned} aria-label={e.pinned ? 'Unpin' : 'Pin'} title={e.pinned ? 'Unpin' : 'Pin: keep it for good'} onclick={() => pin(e)}><Icon name="pin" size={14} /></button>
        <button class="tool" aria-label="Delete" title="Delete" onclick={() => remove(e)}><Icon name="trash" size={14} /></button>
      </li>
    {/each}
  </ul>

  {#if mode === 'popup'}
    <div class="foot"><kbd>↑</kbd><kbd>↓</kbd> choose <kbd>Enter</kbd> paste <kbd>Esc</kbd> close</div>
  {:else if note}
    <p class="note" role="status">{note}</p>
  {/if}
</div>

<style>
  .wrap { display: grid; gap: 10px; min-height: 0; }
  .wrap.popup { height: 100vh; grid-template-rows: auto 1fr auto; padding: 12px; box-sizing: border-box; background: var(--surface-1); }
  .searchbar { display: flex; align-items: center; gap: 8px; padding: 9px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border-strong); color: var(--text-3); }
  .searchbar input { flex: 1; border: 0; background: none; color: var(--text); font: inherit; outline: none; min-width: 0; }
  .searchbar:focus-within { border-color: var(--accent); }
  .empty { margin: 8px 4px; color: var(--text-3); font-size: 13px; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 5px; overflow: auto; align-content: start; }
  .wrap:not(.popup) .list { max-height: 60vh; }
  .clip-row { display: flex; align-items: stretch; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid transparent; }
  .clip-row.active { border-color: var(--accent); background: var(--surface-3); }
  .clip-row.pinned { box-shadow: inset 3px 0 0 var(--accent); }
  .main { flex: 1; min-width: 0; display: grid; gap: 2px; text-align: left; border: 0; background: none; color: inherit; padding: 8px 10px; cursor: pointer; }
  .text { overflow: hidden; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; word-break: break-word; font-size: 13.5px; }
  .meta { color: var(--text-3); font-size: 11.5px; }
  .sw { display: inline-block; width: 12px; height: 12px; border-radius: 4px; margin-right: 6px; vertical-align: -1px; border: 1px solid var(--border-strong); }
  .act { flex: none; align-self: center; margin-right: 4px; border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text-2); border-radius: 99px; padding: 3px 11px; font: inherit; font-size: 12px; cursor: pointer; }
  .act:hover { background: var(--surface-3); color: var(--text); }
  .chip { flex: none; align-self: center; margin-right: 6px; padding: 2px 9px; border-radius: 99px; background: var(--glow-peach); color: var(--accent-text); font-size: 12.5px; font-weight: 700; }
  .tool { flex: none; width: 30px; border: 0; background: none; color: var(--text-3); cursor: pointer; display: grid; place-items: center; }
  .tool:hover { color: var(--text); }
  .tool.on { color: var(--accent-text); }
  .foot { color: var(--text-3); font-size: 11.5px; display: flex; gap: 6px; align-items: center; }
  kbd { font: inherit; padding: 1px 6px; border-radius: 5px; background: var(--surface-3); color: var(--text-2); border: 1px solid var(--border); }
  .note { margin: 0; color: var(--ok); font-size: 12.5px; }
</style>
