<script lang="ts">
  import { onMount } from 'svelte';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import {
    TOOLS,
    TOOL_CATEGORIES,
    TOOL_SETS,
    activeToolSet,
    toolsByCategory,
    withCategory,
    withTool,
    type ToolCategoryId,
  } from '../../lib/tools';
  import Switch from '../../lib/ui/Switch.svelte';
  import GuideDrawer from './GuideDrawer.svelte';

  const picked = $derived(app.settings?.appearance.sidebar_tools ?? null);
  const shown = (id: string) => picked === null || picked.includes(id);
  const count = $derived(TOOLS.filter((t) => shown(t.id)).length);
  const currentSet = $derived(activeToolSet(picked));

  let query = $state('');
  let activeCat = $state<'all' | ToolCategoryId>('all');
  let onlyPinned = $state(false);
  let searchEl = $state<HTMLInputElement | null>(null);
  let guideTopic = $state<string | null>(null);

  onMount(() => {
    try {
      const pending = localStorage.getItem('nomnom.openGuide');
      if (pending) {
        localStorage.removeItem('nomnom.openGuide');
        guideTopic = pending;
      }
    } catch {
      /* optional shortcut into the guide */
    }
  });

  const filteredTools = $derived.by(() => {
    const q = query.trim().toLowerCase();
    return TOOLS.filter((t) => {
      if (activeCat !== 'all' && t.category !== activeCat) return false;
      if (onlyPinned && !shown(t.id)) return false;
      if (!q) return true;
      const cat = TOOL_CATEGORIES.find((c) => c.id === t.category);
      return (
        t.title.toLowerCase().includes(q) ||
        t.desc.toLowerCase().includes(q) ||
        (cat?.title.toLowerCase().includes(q) ?? false)
      );
    });
  });

  const groups = $derived(toolsByCategory(filteredTools));

  const categoryCounts = $derived(
    Object.fromEntries(
      TOOL_CATEGORIES.map((c) => [c.id, TOOLS.filter((t) => t.category === c.id).length]),
    ) as Record<ToolCategoryId, number>,
  );

  function onGlobalKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'f') {
      e.preventDefault();
      searchEl?.focus();
    }
  }
</script>

<svelte:window onkeydown={onGlobalKey} />

<header class="head">
  <div>
    <h1>Tools</h1>
    <p class="sub">Pick a Tool Set for your workflow or toggle individual tools by category. Hidden tools stay one click away.</p>
  </div>
  <div class="actions">
    <label class="find">
      <Icon name="search" size={15} />
      <input
        bind:this={searchEl}
        type="search"
        placeholder="Search tools"
        aria-label="Search tools"
        bind:value={query}
        onkeydown={(e) => e.key === 'Escape' && (query = '')}
      />
      {#if query}
        <button class="clear" aria-label="Clear search" onclick={() => (query = '')}>
          <Icon name="x" size={12} />
        </button>
      {/if}
    </label>
    <button class="primary" onclick={() => (guideTopic = 'guide-launcher')}>
      <Icon name="book" size={15} /> Complete guide
    </button>
  </div>
</header>

<!-- Curated Tool Sets (Workflow Presets) -->
<section class="sets-section" aria-labelledby="sets-title">
  <div class="sets-head">
    <div class="sets-title-row">
      <h2 id="sets-title">Tool Sets</h2>
      {#if !currentSet}
        <span class="custom-pill">Custom selection</span>
      {/if}
    </div>
    <span class="count" role="status">{count} of {TOOLS.length} tools in the sidebar</span>
  </div>
  <div class="sets-grid">
    {#each TOOL_SETS as preset (preset.id)}
      {@const on = currentSet === preset.id}
      <button
        class="set-card"
        class:on
        aria-pressed={on}
        onclick={() => app.setSidebarTools([...preset.tools])}
      >
        <div class="set-top">
          <span class="set-ico"><Icon name={preset.icon} size={15} /></span>
          <strong>{preset.title}</strong>
          <span class="badge tabular">{preset.tools.length}</span>
        </div>
        <span class="set-desc">{preset.desc}</span>
      </button>
    {/each}
  </div>
</section>

<div class="filter-bar">
  <div class="chips" role="tablist" aria-label="Tool categories">
    <button
      role="tab"
      aria-selected={activeCat === 'all'}
      class="chip"
      class:on={activeCat === 'all'}
      onclick={() => (activeCat = 'all')}
    >
      <Icon name="grid" size={14} />
      <span>All</span>
      <span class="badge tabular">{TOOLS.length}</span>
    </button>
    {#each TOOL_CATEGORIES as cat (cat.id)}
      <button
        role="tab"
        aria-selected={activeCat === cat.id}
        class="chip"
        class:on={activeCat === cat.id}
        onclick={() => (activeCat = cat.id)}
      >
        <Icon name={cat.icon} size={14} />
        <span>{cat.title}</span>
        <span class="badge tabular">{categoryCounts[cat.id]}</span>
      </button>
    {/each}
  </div>

  <button
    class="chip pin-filter"
    class:on={onlyPinned}
    aria-pressed={onlyPinned}
    onclick={() => (onlyPinned = !onlyPinned)}
  >
    <Icon name="pin" size={13} />
    <span>In sidebar only</span>
  </button>
</div>

{#if groups.length === 0}
  <div class="empty">
    <p>No tools match the current filter.</p>
    <button
      class="plain"
      onclick={() => {
        query = '';
        activeCat = 'all';
        onlyPinned = false;
      }}
    >
      Clear filters
    </button>
  </div>
{:else}
  <div class="categories">
    {#each groups as g (g.category.id)}
      {@const inGroup = g.tools.filter((t) => shown(t.id)).length}
      {@const allInGroup = inGroup === g.tools.length}
      <section class="cat-section" aria-labelledby={`cat-${g.category.id}`}>
        <div class="cat-head">
          <div class="cat-title-wrap">
            <span class="cat-ico"><Icon name={g.category.icon} size={16} /></span>
            <div>
              <div class="cat-name-row">
                <h2 id={`cat-${g.category.id}`}>{g.category.title}</h2>
                <span class="cat-count tabular">{inGroup} / {g.tools.length}</span>
              </div>
              <p class="cat-desc">{g.category.desc}</p>
            </div>
          </div>
          <button
            class="cat-toggle"
            onclick={() => app.setSidebarTools(withCategory(picked, g.category.id, !allInGroup))}
          >
            {allInGroup ? 'Hide group' : 'Show group'}
          </button>
        </div>

        <div class="grid">
          {#each g.tools as t (t.id)}
            <article class="tool" class:off={!shown(t.id)} style:--tile-glow={t.tint}>
              <div class="top">
                <div class="tile-icon"><Icon name={t.icon} size={20} /></div>
                <Switch
                  checked={shown(t.id)}
                  label={`Show ${t.title} in the sidebar`}
                  onchange={(v) => app.setSidebarTools(withTool(picked, t.id, v))}
                />
              </div>
              <h3>{t.title}</h3>
              <p class="desc">{t.desc}</p>
              <div class="foot">
                <button class="guide-btn" title={`How to use ${t.title}`} onclick={() => (guideTopic = t.id)}>
                  <Icon name="book" size={13} /> How to use
                </button>
                <button class="open" onclick={() => app.go(t.id)}>Open <Icon name="arrow" size={14} /></button>
              </div>
            </article>
          {/each}
        </div>
      </section>
    {/each}
  </div>
{/if}

{#if guideTopic}
  <GuideDrawer initialId={guideTopic} onclose={() => (guideTopic = null)} />
{/if}

<style>
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 20px;
    margin-bottom: 16px;
    flex-wrap: wrap;
  }
  h1 { margin: 0 0 4px; }
  .sub { margin: 0; color: var(--text-2); max-width: 560px; font-size: 13.5px; }
  .actions { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }

  .find {
    display: flex;
    align-items: center;
    gap: 8px;
    width: min(230px, 100%);
    padding: 6px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text-3);
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .find:focus-within { border-color: var(--accent); background: var(--surface-2); color: var(--text); }
  .find input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    user-select: text;
    font-size: 13px;
  }
  .find input::placeholder { color: var(--text-3); }
  .clear {
    display: grid;
    place-items: center;
    width: 18px;
    height: 18px;
    border: 0;
    border-radius: var(--radius-xs);
    background: var(--surface-3);
    color: var(--text-2);
    cursor: pointer;
  }

  /* Curated Tool Sets */
  .sets-section {
    display: grid;
    gap: 10px;
    margin-bottom: 18px;
    padding: 14px 16px;
    border-radius: var(--radius-m);
    background: var(--surface-1);
    border: 1px solid var(--border);
    box-shadow: var(--shadow-card);
  }
  .sets-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    flex-wrap: wrap;
  }
  .sets-title-row { display: flex; align-items: center; gap: 8px; }
  .sets-head h2 { margin: 0; font-size: 14.5px; }
  .custom-pill {
    font-size: 11px;
    font-weight: 600;
    padding: 1px 7px;
    border-radius: var(--radius-xs);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--text-3);
  }
  .sets-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(195px, 1fr));
    gap: 8px;
  }
  .set-card {
    display: grid;
    gap: 5px;
    text-align: left;
    padding: 9px 11px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-2);
    cursor: pointer;
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .set-card:hover { border-color: var(--border-strong); background: var(--surface-3); }
  .set-card.on {
    background: var(--accent-soft);
    border-color: var(--accent);
  }
  .set-top { display: flex; align-items: center; gap: 7px; font-size: 13px; color: var(--text); }
  .set-top strong { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .set-ico { color: var(--accent-text); display: grid; place-items: center; flex: none; }
  .set-desc { font-size: 11.5px; color: var(--text-3); line-height: 1.38; }

  .filter-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    flex-wrap: wrap;
    margin-bottom: 18px;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 600;
    cursor: pointer;
    transition: background var(--dur-1), border-color var(--dur-1), color var(--dur-1);
  }
  .chip:hover { background: var(--surface-2); border-color: var(--border-strong); color: var(--text); }
  .chip.on {
    background: var(--accent-soft);
    border-color: var(--border-glow);
    color: var(--accent-text);
  }
  .badge {
    font-size: 11px;
    padding: 0 5px;
    border-radius: var(--radius-xs);
    background: var(--surface-2);
    color: var(--text-3);
  }
  .chip.on .badge,
  .set-card.on .badge {
    background: var(--surface-1);
    color: var(--accent-text);
  }

  .count { color: var(--text-3); font-size: 12.5px; }
  .pin-filter { padding: 5px 10px; font-size: 12px; }

  .categories { display: grid; gap: 24px; }
  .cat-section { display: grid; gap: 12px; }
  .cat-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
  }
  .cat-title-wrap { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .cat-ico {
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--accent-text);
    display: grid;
    place-items: center;
  }
  .cat-name-row { display: flex; align-items: baseline; gap: 8px; }
  .cat-name-row h2 { margin: 0; font-size: 15.5px; }
  .cat-count {
    font-size: 11.5px;
    font-weight: 600;
    color: var(--text-3);
    background: var(--surface-2);
    padding: 1px 7px;
    border-radius: var(--radius-xs);
  }
  .cat-desc { margin: 2px 0 0; font-size: 12.5px; color: var(--text-3); }
  .cat-toggle {
    padding: 5px 11px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .cat-toggle:hover { background: var(--surface-2); border-color: var(--border-strong); color: var(--text); }

  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(244px, 1fr)); gap: 10px; }
  .tool {
    display: flex; flex-direction: column; gap: 6px; padding: 14px 16px;
    background: var(--surface-1);
    border: 1px solid var(--border); border-radius: var(--radius-m); box-shadow: var(--shadow-card);
    transition: opacity var(--dur-1), border-color var(--dur-1), background var(--dur-1);
  }
  .tool.off { opacity: 0.7; border-style: dashed; background: transparent; }
  .tool:hover { border-color: var(--border-strong); background: var(--surface-2); }
  .top { display: flex; justify-content: space-between; align-items: center; }
  .tile-icon { display: grid; place-items: center; width: 34px; height: 34px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); color: var(--accent-text); }
  h3 { margin: 6px 0 0; font-size: 14.5px; }
  .desc { margin: 0; color: var(--text-3); font-size: 12.5px; flex: 1; line-height: 1.45; }
  .foot { display: flex; justify-content: space-between; align-items: center; margin-top: 8px; gap: 8px; }
  .guide-btn {
    display: inline-flex; align-items: center; gap: 5px;
    border: 1px solid var(--border); background: var(--surface-2); color: var(--text-2);
    border-radius: var(--radius-xs); padding: 3px 8px; font-size: 12px; font-weight: 600; cursor: pointer;
  }
  .guide-btn:hover { background: var(--surface-3); border-color: var(--border-strong); color: var(--text); }
  .open { display: inline-flex; align-items: center; gap: 5px; border: 0; background: none; color: var(--accent-text); font-weight: 600; font-size: 13px; cursor: pointer; padding: 3px 2px; }
  .open:hover { text-decoration: underline; }
  button.plain, button.primary {
    display: inline-flex; align-items: center; gap: 6px;
    padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; font-size: 13px;
  }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  button.primary:hover { background: var(--accent-hover); }

  .empty {
    display: grid;
    justify-items: start;
    gap: 10px;
    padding: 24px;
    border-radius: var(--radius-m);
    background: var(--surface-1);
    border: 1px solid var(--border);
    color: var(--text-2);
  }
  .empty p { margin: 0; }
</style>
