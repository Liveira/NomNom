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
  .top { display: flex; align-items: flex-end; justify-content: space-between; gap: 20px; flex-wrap: wrap; margin-bottom: 18px; }
  .sub { margin: 4px 0 0; color: var(--text-2); }
  .find {
    display: flex; align-items: center; gap: 8px; width: min(300px, 100%); padding: 8px 12px;
    border-radius: var(--radius-pill); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text-3);
    transition: border-color var(--dur-2), box-shadow var(--dur-2), width var(--dur-3) var(--ease-out);
  }
  .find:focus-within { border-color: var(--accent); box-shadow: 0 0 0 3px var(--glow-peach); color: var(--accent-text); width: min(360px, 100%); }
  .find input { flex: 1; min-width: 0; border: 0; outline: none; background: transparent; color: var(--text); user-select: text; }
  .find input::placeholder { color: var(--text-3); }
  .clear { display: grid; place-items: center; width: 22px; height: 22px; border: 0; border-radius: 50%; background: var(--surface-3); color: var(--text-2); cursor: pointer; }

  .layout { display: grid; grid-template-columns: 214px minmax(0, 1fr); gap: 22px; align-items: start; }
  @media (max-width: 760px) { .layout { grid-template-columns: 1fr; } }

  .snav { position: sticky; top: 0; }
  .tabs {
    position: relative; display: grid; gap: 4px; padding: 8px; border-radius: var(--radius-l);
    background: var(--surface-1); border: 1px solid var(--border-strong); box-shadow: var(--shadow-soft);
  }
  .pill {
    position: absolute; left: 8px; right: 8px; top: 8px; height: 42px; border-radius: var(--radius-s);
    background: linear-gradient(135deg, var(--surface-3), var(--surface-2)); border: 1px solid var(--border-glow);
    box-shadow: 0 0 20px var(--glow-peach);
    transform: translateY(calc(var(--i) * 46px));
    transition: transform var(--dur-3) var(--ease-spring), opacity var(--dur-2);
  }
  .pill.hide { opacity: 0; }
  .tab {
    position: relative; z-index: 1; display: flex; align-items: center; gap: 11px; height: 42px; padding: 0 10px;
    border: 0; border-radius: var(--radius-s); background: transparent; color: var(--text-2); cursor: pointer; text-align: left; font-weight: 600;
  }
  .tab:hover { color: var(--text); }
  .tab[aria-selected='true'] { color: var(--text); }
  .ico { display: grid; place-items: center; width: 26px; height: 26px; border-radius: 9px; transition: transform var(--dur-2) var(--ease-spring), background var(--dur-2); }
  .tab:hover .ico { transform: scale(1.12) rotate(-6deg); }
  .tab[aria-selected='true'] .ico { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); }
  .none { margin: 10px 4px; color: var(--text-3); font-size: 12.5px; }

  .pane { min-width: 0; }
  .pane-in { animation: pane-in var(--dur-4) var(--ease-out); }
  @keyframes pane-in { from { opacity: 0; transform: translateX(18px); } to { opacity: 1; transform: none; } }
  .sec + .sec { margin-top: 30px; }
  .phead { display: flex; align-items: center; gap: 14px; margin-bottom: 16px; }
  .phead h2 { font-size: 21px; }
  .phead p { margin: 2px 0 0; color: var(--text-2); font-size: 13px; }
  .pico {
    display: grid; place-items: center; width: 46px; height: 46px; border-radius: 16px; flex: none;
    color: var(--on-accent); background: linear-gradient(135deg, var(--accent), var(--blush));
    box-shadow: 0 6px 20px var(--glow-peach), inset 0 1px 0 rgba(255, 255, 255, 0.35);
    animation: bob 5s ease-in-out infinite;
  }
  @keyframes bob { 0%, 100% { transform: translateY(0) rotate(-3deg); } 50% { transform: translateY(-3px) rotate(3deg); } }
  .empty { color: var(--text-2); }

  /* One look for every settings card, whichever section it comes from. */
  .pane :global(section.group) {
    max-width: none; margin: 0 0 16px; padding: 20px 22px; border-radius: var(--radius-l);
    background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-soft);
    animation: rise var(--dur-4) var(--ease-out) backwards;
  }
  .pane :global(section.group:nth-of-type(2)) { animation-delay: 70ms; }
  .pane :global(section.group:nth-of-type(3)) { animation-delay: 140ms; }
  .pane :global(section.group:nth-of-type(n + 4)) { animation-delay: 200ms; }
  .pane :global(section.group > h3) { display: flex; align-items: center; gap: 9px; margin-bottom: 14px; font-size: 15.5px; }
  .pane :global(section.group > h3)::before { content: ''; width: 4px; height: 16px; border-radius: 3px; background: linear-gradient(180deg, var(--accent), var(--blush)); }
  .pane :global(section.group .row) { margin-inline: -10px; padding: 11px 10px; border-radius: var(--radius-s); transition: background var(--dur-2); }
  .pane :global(section.group .row:hover) { background: color-mix(in srgb, var(--surface-2) 70%, transparent); }
  .pane :global(section.group .label) { font-weight: 700; }
  .pane :global(section.group .hint) { line-height: 1.45; }
</style>
