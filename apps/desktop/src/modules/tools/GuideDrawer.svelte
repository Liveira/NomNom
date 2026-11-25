<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { CORE_GUIDES, TOOL_GUIDES, type GuideEntry } from '../../lib/tool-guides';
  import { TOOLS, TOOL_CATEGORIES } from '../../lib/tools';

  let {
    initialId = 'guide-launcher',
    onclose,
  }: {
    initialId?: string;
    onclose: () => void;
  } = $props();

  const ALL_ENTRIES: GuideEntry[] = [
    ...CORE_GUIDES,
    ...TOOLS.map((t) => ({
      id: t.id,
      title: t.title,
      icon: t.icon,
      category: t.category,
      route: t.id,
      ...TOOL_GUIDES[t.id],
    })),
  ];

  let selectedId = $state('guide-launcher');
  let query = $state('');

  $effect(() => {
    selectedId = initialId;
  });

  const filtered = $derived.by(() => {
    const q = query.trim().toLowerCase();
    if (!q) return ALL_ENTRIES;
    return ALL_ENTRIES.filter(
      (e) =>
        e.title.toLowerCase().includes(q) ||
        e.summary.toLowerCase().includes(q) ||
        e.steps.some((s) => s.toLowerCase().includes(q)) ||
        (e.shortcut?.toLowerCase().includes(q) ?? false),
    );
  });

  const grouped = $derived.by(() => {
    const sections: { id: string; title: string; items: GuideEntry[] }[] = [];
    const core = filtered.filter((e) => e.category === 'core');
    if (core.length > 0) sections.push({ id: 'core', title: 'Core & Shortcuts', items: core });
    for (const cat of TOOL_CATEGORIES) {
      const items = filtered.filter((e) => e.category === cat.id);
      if (items.length > 0) sections.push({ id: cat.id, title: cat.title, items });
    }
    return sections;
  });

  const active = $derived(
    ALL_ENTRIES.find((e) => e.id === selectedId) ?? filtered[0] ?? ALL_ENTRIES[0],
  );

  function openRoute(route: string) {
    onclose();
    app.go(route);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="backdrop" role="presentation" onclick={(e) => e.target === e.currentTarget && onclose()}>
  <div
    class="modal"
    role="dialog"
    aria-modal="true"
    aria-labelledby="guide-title"
    tabindex="-1"
  >
    <header class="mhead">
      <div class="mhead-l">
        <span class="mico"><Icon name="book" size={18} /></span>
        <div>
          <h2 id="guide-title">NomNom Guide</h2>
          <p class="msub">Step-by-step instructions and shortcuts for every tool.</p>
        </div>
      </div>
      <button class="xbtn" aria-label="Close guide" onclick={onclose}>
        <Icon name="x" size={16} />
      </button>
    </header>

    <div class="mbody">
      <aside class="gnav" aria-label="Guide topics">
        <label class="gfind">
          <Icon name="search" size={14} />
          <input
            type="search"
            placeholder="Search guide"
            aria-label="Search guide"
            bind:value={query}
          />
          {#if query}
            <button class="clear" aria-label="Clear search" onclick={() => (query = '')}>
              <Icon name="x" size={11} />
            </button>
          {/if}
        </label>

        <div class="glist">
          {#each grouped as sec (sec.id)}
            <div class="gsec">{sec.title}</div>
            {#each sec.items as entry (entry.id)}
              <button
                class="gitem"
                class:on={active.id === entry.id}
                onclick={() => (selectedId = entry.id)}
              >
                <Icon name={entry.icon} size={15} />
                <span>{entry.title}</span>
              </button>
            {/each}
          {/each}
          {#if grouped.length === 0}
            <p class="gempty">No guide topic matches “{query}”.</p>
          {/if}
        </div>
      </aside>

      <article class="gdoc">
        <div class="doc-top">
          <div class="doc-title-wrap">
            <span class="doc-ico"><Icon name={active.icon} size={22} /></span>
            <div>
              <h3>{active.title}</h3>
              <p class="doc-sum">{active.summary}</p>
            </div>
          </div>
          {#if active.route}
            <button class="primary" onclick={() => openRoute(active.route!)}>
              Open tool <Icon name="arrow" size={14} />
            </button>
          {/if}
        </div>

        {#if active.shortcut || active.requires}
          <div class="badges">
            {#if active.shortcut}
              <span class="meta-badge">
                <Icon name="keyboard" size={13} />
                <span>Shortcut:</span>
                <kbd translate="no">{active.shortcut}</kbd>
              </span>
            {/if}
            {#if active.requires}
              <span class="meta-badge">
                <Icon name="info" size={13} />
                <span>Requires: {active.requires}</span>
              </span>
            {/if}
          </div>
        {/if}

        <section class="doc-block">
          <h4>How to use</h4>
          <ol class="steps">
            {#each active.steps as step, i (i)}
              <li>
                <span class="step-num tabular">{i + 1}</span>
                <span>{step}</span>
              </li>
            {/each}
          </ol>
        </section>

        <section class="doc-block">
          <h4>Tips</h4>
          <ul class="tips">
            {#each active.tips as tip, i (i)}
              <li>{tip}</li>
            {/each}
          </ul>
        </section>
      </article>
    </div>
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 80;
    background: rgba(6, 5, 12, 0.62);
    backdrop-filter: blur(4px);
    display: grid;
    place-items: center;
    padding: 20px;
  }
  .modal {
    width: min(920px, 100%);
    height: min(640px, calc(100vh - 48px));
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-m);
    box-shadow: var(--shadow-pop);
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }
  .mhead {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    background: var(--surface-1);
    flex: none;
  }
  .mhead-l { display: flex; align-items: center; gap: 11px; }
  .mico {
    width: 34px;
    height: 34px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--accent-text);
    display: grid;
    place-items: center;
  }
  .mhead h2 { margin: 0; font-size: 16px; }
  .msub { margin: 1px 0 0; font-size: 12.5px; color: var(--text-3); }
  .xbtn {
    width: 30px;
    height: 30px;
    border: 1px solid var(--border);
    border-radius: var(--radius-s);
    background: var(--surface-2);
    color: var(--text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
  }
  .xbtn:hover { background: var(--surface-3); color: var(--text); }

  .mbody {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 240px minmax(0, 1fr);
  }
  @media (max-width: 720px) {
    .mbody { grid-template-columns: 1fr; grid-template-rows: 190px minmax(0, 1fr); }
    .gnav { border-right: 0; border-bottom: 1px solid var(--border); }
  }

  .gnav {
    display: flex;
    flex-direction: column;
    min-height: 0;
    border-right: 1px solid var(--border);
    background: var(--bg);
    padding: 10px;
    gap: 8px;
  }
  .gfind {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 9px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text-3);
    flex: none;
  }
  .gfind:focus-within { border-color: var(--accent); color: var(--text); }
  .gfind input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 12.5px;
    user-select: text;
  }
  .clear {
    width: 17px;
    height: 17px;
    border: 0;
    border-radius: var(--radius-xs);
    background: var(--surface-3);
    color: var(--text-2);
    display: grid;
    place-items: center;
    cursor: pointer;
  }

  .glist {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    display: grid;
    align-content: start;
    gap: 2px;
  }
  .gsec {
    padding: 9px 8px 3px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
  }
  .gitem {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 6px 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-xs);
    background: transparent;
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 500;
    text-align: left;
    cursor: pointer;
  }
  .gitem:hover { background: var(--surface-2); color: var(--text); }
  .gitem.on {
    background: var(--accent-soft);
    border-color: var(--border-glow);
    color: var(--accent-text);
    font-weight: 700;
  }
  .gempty { padding: 10px 8px; color: var(--text-3); font-size: 12.5px; margin: 0; }

  .gdoc {
    overflow-y: auto;
    padding: 20px 24px 28px;
    display: grid;
    align-content: start;
    gap: 18px;
    user-select: text;
  }
  .doc-top {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
    padding-bottom: 14px;
    border-bottom: 1px solid var(--border);
  }
  .doc-title-wrap { display: flex; align-items: flex-start; gap: 12px; min-width: 0; flex: 1; }
  .doc-ico {
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--accent-text);
    display: grid;
    place-items: center;
  }
  .doc-top h3 { margin: 0 0 4px; font-size: 18px; }
  .doc-sum { margin: 0; color: var(--text-2); font-size: 13.5px; line-height: 1.5; }

  .badges { display: flex; flex-wrap: wrap; gap: 8px; }
  .meta-badge {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 5px 10px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 12px;
    color: var(--text-2);
  }
  .meta-badge kbd {
    font-family: var(--font-mono);
    font-size: 11.5px;
    font-weight: 700;
    color: var(--accent-text);
    background: var(--surface-1);
    padding: 1px 6px;
    border-radius: var(--radius-xs);
    border: 1px solid var(--border-strong);
  }

  .doc-block { display: grid; gap: 9px; }
  .doc-block h4 {
    margin: 0;
    font-size: 12px;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--text-3);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 9px;
  }
  .steps li {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 13px;
    line-height: 1.5;
  }
  .step-num {
    width: 22px;
    height: 22px;
    flex: none;
    border-radius: var(--radius-xs);
    background: var(--accent);
    color: var(--on-accent);
    font-size: 11.5px;
    font-weight: 700;
    display: grid;
    place-items: center;
  }
  .tips {
    margin: 0;
    padding-left: 18px;
    display: grid;
    gap: 6px;
    color: var(--text-2);
    font-size: 13px;
    line-height: 1.5;
  }
  .primary {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 7px 14px;
    border: 0;
    border-radius: var(--radius-s);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    flex: none;
  }
  .primary:hover { background: var(--accent-hover); }
</style>
