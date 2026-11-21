<script lang="ts">
  import type { Component } from 'svelte';
  import { tick } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import AboutSettings from './AboutSettings.svelte';
  import ActivitySettings from './ActivitySettings.svelte';
  import AppearanceSettings from './AppearanceSettings.svelte';
  import BrowserSettings from './BrowserSettings.svelte';
  import DownloaderSettings from './DownloaderSettings.svelte';
  import GeneralSettings from './GeneralSettings.svelte';
  import OrganizerSettings from './OrganizerSettings.svelte';
  import Shortcuts from './Shortcuts.svelte';
  import SystemSettings from './SystemSettings.svelte';
  import UpdateSettings from './UpdateSettings.svelte';
  import { SECTIONS, matchSections } from './sections';

  const views: Record<string, Component> = {
    appearance: AppearanceSettings,
    general: GeneralSettings,
    downloads: DownloaderSettings,
    organizer: OrganizerSettings,
    playtime: ActivitySettings,
    browser: BrowserSettings,
    shortcuts: Shortcuts,
    system: SystemSettings,
    updates: UpdateSettings,
    about: AboutSettings,
  };

  let active = $state('appearance');
  let query = $state('');
  let search = $state<HTMLInputElement | null>(null);

  const found = $derived(matchSections(query));
  const searching = $derived(query.trim().length > 0);
  const shown = $derived(searching ? found : SECTIONS.filter((s) => s.id === active));
  const index = $derived(Math.max(0, found.findIndex((s) => s.id === active)));

  /** Up and Down move through the list; the section opens as you go. */
  async function keys(e: KeyboardEvent) {
    const list = found;
    const at = list.findIndex((s) => s.id === active);
    let next = -1;
    if (e.key === 'ArrowDown') next = Math.min(list.length - 1, at + 1);
    else if (e.key === 'ArrowUp') next = Math.max(0, at - 1);
    else if (e.key === 'Home') next = 0;
    else if (e.key === 'End') next = list.length - 1;
    if (next < 0) return;
    e.preventDefault();
    active = list[next].id;
    await tick();
    (e.currentTarget as HTMLElement).querySelector<HTMLElement>('[aria-selected=true]')?.focus();
  }

  function open(id: string) {
    active = id;
    query = '';
  }

  function onGlobalKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      search?.focus();
    }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

<header class="top">
  <div>
    <h1>Settings</h1>
    <p class="sub">Make NomNom yours. Changes are saved as you make them.</p>
  </div>
  <label class="find">
    <Icon name="search" size={16} />
    <input
      bind:this={search}
      type="search"
      placeholder="Search settings"
      aria-label="Search settings"
      bind:value={query}
      onkeydown={(e) => e.key === 'Escape' && (query = '')}
    />
    {#if query}<button class="clear" aria-label="Clear search" onclick={() => (query = '')}><Icon name="x" size={13} /></button>{/if}
  </label>
</header>

<div class="layout">
  <nav class="snav" aria-label="Settings sections">
    <div class="tabs" role="tablist" aria-orientation="vertical" tabindex="-1" onkeydown={keys}>
      {#if !searching || found.length > 0}
        <i class="pill" style:--i={index} class:hide={searching && !found.some((s) => s.id === active)}></i>
      {/if}
      {#each found as s (s.id)}
        <button
          class="tab"
          role="tab"
          id={`tab-${s.id}`}
          aria-selected={active === s.id}
          aria-controls="pane"
          tabindex={active === s.id ? 0 : -1}
          onclick={() => open(s.id)}
        >
          <span class="ico"><Icon name={s.icon} size={17} /></span>
          <span>{s.name}</span>
        </button>
      {/each}
    </div>
    {#if found.length === 0}
      <p class="none" role="status">Nothing matches “{query}”.</p>
    {/if}
  </nav>

  <div class="pane" id="pane" role="tabpanel" aria-labelledby={`tab-${active}`}>
    {#key searching ? `q:${query}` : active}
      <div class="pane-in">
        {#each shown as s (s.id)}
          {@const View = views[s.id]}
          <div class="sec">
            <header class="phead">
              <span class="pico"><Icon name={s.icon} size={20} /></span>
              <div>
                <h2>{s.name}</h2>
                <p>{s.blurb}</p>
              </div>
            </header>
            <View />
          </div>
        {/each}
        {#if searching && shown.length === 0}
          <p class="empty">No setting is called that. Try a shorter word, like “theme”, “shortcut” or “update”.</p>
        {/if}
      </div>
    {/key}
  </div>
</div>

<style>
  .top { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; flex-wrap: wrap; margin-bottom: 20px; padding-bottom: 16px; border-bottom: 1px solid var(--border); }
  .sub { margin: 4px 0 0; color: var(--text-2); font-size: 13.5px; }
  .find {
    display: flex; align-items: center; gap: 8px; width: min(280px, 100%); padding: 7px 12px;
    border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text-3);
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .find:focus-within { border-color: var(--accent); background: var(--surface-2); color: var(--text); }
  .find input { flex: 1; min-width: 0; border: 0; outline: none; background: transparent; color: var(--text); user-select: text; font-size: 13px; }
  .find input::placeholder { color: var(--text-3); }
  .clear { display: grid; place-items: center; width: 20px; height: 20px; border: 0; border-radius: 50%; background: var(--surface-3); color: var(--text-2); cursor: pointer; }

  .layout { display: grid; grid-template-columns: 204px minmax(0, 1fr); gap: 24px; align-items: start; }
  @media (max-width: 760px) { .layout { grid-template-columns: 1fr; } }

  .snav { position: sticky; top: 0; }
  .tabs {
    position: relative; display: grid; gap: 2px;
  }
  .pill {
    position: absolute; left: 0; right: 0; top: 0; height: 36px; border-radius: var(--radius-s);
    background: var(--surface-2); border: 1px solid var(--border);
    transform: translateY(calc(var(--i) * 38px));
    transition: transform var(--dur-2) var(--ease-out), opacity var(--dur-1);
  }
  .pill.hide { opacity: 0; }
  .tab {
    position: relative; z-index: 1; display: flex; align-items: center; gap: 10px; height: 36px; padding: 0 10px;
    border: 0; border-radius: var(--radius-s); background: transparent; color: var(--text-2); cursor: pointer; text-align: left; font-weight: 500; font-size: 13.5px;
    transition: color var(--dur-1);
  }
  .tab:hover { color: var(--text); }
  .tab[aria-selected='true'] { color: var(--text); font-weight: 600; }
  .ico { display: grid; place-items: center; width: 22px; height: 22px; border-radius: 6px; color: var(--text-3); transition: color var(--dur-1); }
  .tab:hover .ico { color: var(--text-2); }
  .tab[aria-selected='true'] .ico { color: var(--accent-text); }
  .none { margin: 10px 4px; color: var(--text-3); font-size: 12.5px; }

  .pane { min-width: 0; }
  .pane-in { animation: pane-in var(--dur-2) var(--ease-out); }
  @keyframes pane-in { from { opacity: 0; } to { opacity: 1; } }
  .sec + .sec { margin-top: 28px; }
  .phead { display: flex; align-items: center; gap: 12px; margin-bottom: 14px; }
  .phead h2 { font-size: 18px; }
  .phead p { margin: 2px 0 0; color: var(--text-2); font-size: 13px; }
  .pico {
    display: grid; place-items: center; width: 36px; height: 36px; border-radius: var(--radius-s); flex: none;
    color: var(--accent-text); background: var(--surface-2); border: 1px solid var(--border);
  }
  .empty { color: var(--text-2); }

  /* One look for every settings card, whichever section it comes from. */
  .pane :global(section.group) {
    max-width: none; margin: 0 0 14px; padding: 18px 20px; border-radius: var(--radius-m);
    background: var(--surface-1); border: 1px solid var(--border); box-shadow: var(--shadow-card);
  }
  .pane :global(section.group > h3) { margin-bottom: 12px; font-size: 14.5px; font-weight: 600; }
  .pane :global(section.group .row) { margin-inline: -8px; padding: 10px 8px; border-radius: var(--radius-s); transition: background var(--dur-1); }
  .pane :global(section.group .row:hover) { background: var(--surface-2); }
  .pane :global(section.group .label) { font-weight: 600; font-size: 13.5px; }
  .pane :global(section.group .hint) { line-height: 1.45; }
</style>
