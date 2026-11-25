<script lang="ts">
  import { tick } from 'svelte';
  import { app } from '../../app/state.svelte';
  import {
    CHANGE_KINDS,
    RELEASES,
    changelogStats,
    filterReleases,
    pickText,
    type ChangeKind,
    type ReleaseNote,
  } from '../../lib/changelog';
  import { i18n } from '../../lib/i18n/index.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { updateIpc } from '../../lib/ipc';

  let installedVersion = $state('');
  let query = $state('');
  let kindFilter = $state<ChangeKind | 'all'>('all');
  let activeVersion = $state(RELEASES[0]?.version ?? '');
  let expanded = $state<Record<string, boolean>>(
    Object.fromEntries(RELEASES.map((r, idx) => [r.version, idx < 3])),
  );

  void updateIpc
    .version()
    .then((v) => (installedVersion = v))
    .catch(() => {});

  const lang = $derived(i18n.lang);
  const stats = $derived(changelogStats(RELEASES));
  const filtered = $derived(filterReleases(query, kindFilter, RELEASES));
  const isFiltering = $derived(query.trim().length > 0 || kindFilter !== 'all');
  const allExpanded = $derived(filtered.length > 0 && filtered.every((r) => isFiltering || expanded[r.version]));

  const KIND_META: Record<ChangeKind, { label: string; icon: string; cls: string }> = {
    added: { label: 'New', icon: 'plus', cls: 'kind-added' },
    improved: { label: 'Improved', icon: 'arrow-up', cls: 'kind-improved' },
    design: { label: 'Design', icon: 'palette', cls: 'kind-design' },
    fixed: { label: 'Fixed', icon: 'check', cls: 'kind-fixed' },
  };

  function isOpen(version: string): boolean {
    return isFiltering || (expanded[version] ?? false);
  }

  function toggleRelease(version: string) {
    if (isFiltering) return;
    expanded = { ...expanded, [version]: !expanded[version] };
  }

  function toggleAll() {
    const next = !allExpanded;
    const copy = { ...expanded };
    for (const r of filtered) copy[r.version] = next;
    expanded = copy;
  }

  async function jumpTo(version: string) {
    activeVersion = version;
    expanded = { ...expanded, [version]: true };
    await tick();
    const el = document.getElementById(`rel-${version}`);
    el?.scrollIntoView({ behavior: 'smooth', block: 'start' });
  }

  function countsOf(r: ReleaseNote): { kind: ChangeKind; n: number }[] {
    return CHANGE_KINDS.map((kind) => ({
      kind,
      n: r.changes.filter((c) => c.kind === kind).length,
    })).filter((x) => x.n > 0);
  }

  function openTool(toolId: string) {
    if (toolId === 'changelog') return;
    app.go(toolId);
  }

  function openGuide() {
    try {
      localStorage.setItem('nomnom.openGuide', 'guide-launcher');
    } catch {
      /* ignore */
    }
    app.go('tools');
  }
</script>

<header class="studio-hero hero">
  <div class="hero-copy">
    <span class="eyebrow"><Icon name="scroll-text" size={14} /> Release history</span>
    <h1>Patch notes</h1>
    <p>Everything added, refined and fixed across every NomNom release, with quick links to try each tool.</p>
  </div>

  <div class="hero-right">
    <div class="meta-strip">
      <div class="stat">
        <span class="stat-val tabular" translate="no">v{installedVersion || RELEASES[0]?.version}</span>
        <span class="stat-lbl">Installed</span>
      </div>
      <div class="stat">
        <span class="stat-val tabular">{stats.versions}</span>
        <span class="stat-lbl">Releases</span>
      </div>
      <div class="stat">
        <span class="stat-val tabular">{stats.totalChanges}</span>
        <span class="stat-lbl">Highlights</span>
      </div>
    </div>
    <div class="hero-actions">
      <button class="ghost-btn" onclick={openGuide}>
        <Icon name="book" size={14} />
        <span>Complete guide</span>
      </button>
      <a class="ghost-btn" href="https://github.com/Liveira/NomNom/releases" target="_blank" rel="noreferrer">
        <Icon name="external" size={14} />
        <span>GitHub Releases</span>
      </a>
    </div>
  </div>
</header>

<section class="toolbar" aria-label="Filter patch notes">
  <label class="search-box">
    <Icon name="search" size={15} />
    <input
      type="search"
      placeholder="Search versions, tools or features"
      aria-label="Search versions, tools or features"
      bind:value={query}
      onkeydown={(e) => e.key === 'Escape' && (query = '')}
    />
    {#if query}
      <button class="clear-btn" aria-label="Clear search" onclick={() => (query = '')}>
        <Icon name="x" size={13} />
      </button>
    {/if}
  </label>

  <div class="filter-row" role="group" aria-label="Filter by change type">
    <button class="chip" class:active={kindFilter === 'all'} onclick={() => (kindFilter = 'all')}>
      <span>All</span>
      <span class="chip-count tabular">{stats.totalChanges}</span>
    </button>
    {#each CHANGE_KINDS as kind (kind)}
      {@const meta = KIND_META[kind]}
      <button
        class={`chip ${meta.cls}`}
        class:active={kindFilter === kind}
        onclick={() => (kindFilter = kindFilter === kind ? 'all' : kind)}
      >
        <Icon name={meta.icon} size={12} />
        <span>{meta.label}</span>
        <span class="chip-count tabular">{stats[kind]}</span>
      </button>
    {/each}

    {#if !isFiltering && filtered.length > 0}
      <button class="chip subtle" onclick={toggleAll}>
        <span>{allExpanded ? 'Collapse older' : 'Expand all'}</span>
      </button>
    {/if}
  </div>
</section>

<!-- Compact horizontal version jumper on medium/narrow viewports -->
<nav class="jump-strip" aria-label="Jump to version">
  {#each filtered as r (r.version)}
    <button
      class="jump-pill"
      class:active={activeVersion === r.version}
      onclick={() => void jumpTo(r.version)}
    >
      <span class="tabular" translate="no">v{r.version}</span>
      {#if r.latest}<span class="dot-live" aria-hidden="true"></span>{/if}
    </button>
  {/each}
</nav>

<div class="board">
  <aside class="version-rail" aria-label="Version timeline">
    <div class="rail-head">
      <Icon name="tag" size={13} />
      <span>Versions</span>
    </div>
    <ul>
      {#each filtered as r (r.version)}
        <li>
          <button
            class="rail-btn"
            class:active={activeVersion === r.version}
            onclick={() => void jumpTo(r.version)}
          >
            <div class="rail-top">
              <span class="rail-ver tabular" translate="no">v{r.version}</span>
              {#if r.latest}
                <span class="mini-tag">Latest</span>
              {:else if installedVersion && r.version === installedVersion}
                <span class="mini-tag installed">Installed</span>
              {/if}
            </div>
            <span class="rail-date tabular" translate="no">{r.date}</span>
          </button>
        </li>
      {/each}
    </ul>
  </aside>

  <section class="timeline" aria-label="Release entries">
    {#if filtered.length === 0}
      <div class="empty-state">
        <Icon name="search-x" size={24} />
        <h3>No matching release notes</h3>
        <p>Try another word like “webcam”, “PDF”, “Ctrl+K”, “backup” or “theme”.</p>
        <button
          class="ghost-btn"
          onclick={() => {
            query = '';
            kindFilter = 'all';
          }}
        >
          Clear filters
        </button>
      </div>
    {:else}
      {#each filtered as r (r.version)}
        {@const open = isOpen(r.version)}
        {@const counts = countsOf(r)}
        <article
          id={`rel-${r.version}`}
          class="release"
          class:latest={r.latest}
          class:collapsed={!open}
          onmouseenter={() => (activeVersion = r.version)}
        >
          <div class="rail-node" aria-hidden="true"></div>

          <div class="card">
            <button
              class="card-head"
              aria-expanded={open}
              onclick={() => toggleRelease(r.version)}
            >
              <div class="head-left">
                <div class="ver-row">
                  <span class="ver-badge tabular" translate="no">v{r.version}</span>
                  {#if r.latest}
                    <span class="status-pill latest-pill">Latest</span>
                  {/if}
                  {#if installedVersion && r.version === installedVersion}
                    <span class="status-pill installed-pill">Installed</span>
                  {/if}
                  <span class="date tabular" translate="no">{r.date}</span>
                </div>
                <h2 class="codename" translate="no">{pickText(r.codename, lang)}</h2>
                <p class="summary" translate="no">{pickText(r.summary, lang)}</p>
              </div>

              <div class="head-right">
                <div class="mini-counts">
                  {#each counts as c (c.kind)}
                    {@const meta = KIND_META[c.kind]}
                    <span class={`count-pill ${meta.cls}`}>
                      <span class="tabular">{c.n}</span>
                      <span>{meta.label}</span>
                    </span>
                  {/each}
                </div>
                {#if !isFiltering}
                  <span class="chev" class:open>
                    <Icon name="chevron-down" size={16} />
                  </span>
                {/if}
              </div>
            </button>

            {#if open}
              <div class="card-body">
                {#if r.highlights.length > 0 && !isFiltering}
                  <ul class="highlights" aria-label="Highlights">
                    {#each r.highlights as h, hi (hi)}
                      <li>
                        <Icon name="sparkle" size={13} />
                        <span translate="no">{pickText(h, lang)}</span>
                      </li>
                    {/each}
                  </ul>
                {/if}

                <ul class="changes">
                  {#each r.changes as ch, ci (`${r.version}-${ci}`)}
                    {@const meta = KIND_META[ch.kind]}
                    <li class="change-row">
                      <div class="change-main">
                        <div class="change-top">
                          <span class={`kind-tag ${meta.cls}`}>
                            <Icon name={meta.icon} size={11} />
                            <span>{meta.label}</span>
                          </span>
                          <strong class="change-title" translate="no">{pickText(ch.title, lang)}</strong>
                        </div>
                        <p class="change-detail" translate="no">{pickText(ch.detail, lang)}</p>
                      </div>

                      {#if ch.toolId && ch.toolId !== 'changelog'}
                        <button class="tool-jump" onclick={() => openTool(ch.toolId!)}>
                          <span>{ch.toolLabel ?? ch.toolId}</span>
                          <Icon name="arrow" size={13} />
                        </button>
                      {/if}
                    </li>
                  {/each}
                </ul>
              </div>
            {/if}
          </div>
        </article>
      {/each}
    {/if}
  </section>
</div>

<style>
  .hero {
    flex-wrap: wrap;
    align-items: flex-end;
    gap: 16px;
  }
  .hero-copy {
    min-width: 0;
    flex: 1 1 320px;
  }
  .hero-right {
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 8px;
    flex: none;
  }
  .meta-strip {
    display: flex;
    align-items: stretch;
    gap: 8px;
    flex-wrap: wrap;
  }
  .stat {
    display: grid;
    padding: 6px 12px;
    border-radius: var(--radius-s);
    background: var(--surface-1);
    border: 1px solid var(--border);
    min-width: 78px;
    text-align: right;
  }
  .stat-val {
    font-family: var(--font-mono);
    font-weight: 700;
    font-size: 13.5px;
    color: var(--text);
    line-height: 1.2;
  }
  .stat-lbl {
    font-size: 11px;
    color: var(--text-3);
  }
  .hero-actions {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .ghost-btn {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 6px 11px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text-2);
    font-size: 12.5px;
    font-weight: 600;
    text-decoration: none;
    cursor: pointer;
  }
  .ghost-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }

  .toolbar {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: 10px 14px;
    margin-bottom: 14px;
  }
  .search-box {
    display: flex;
    align-items: center;
    gap: 8px;
    flex: 1 1 240px;
    max-width: 340px;
    padding: 7px 11px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--text-3);
  }
  .search-box:focus-within {
    border-color: var(--accent);
    background: var(--surface-2);
    color: var(--text);
  }
  .search-box input {
    flex: 1;
    min-width: 0;
    border: 0;
    outline: none;
    background: transparent;
    color: var(--text);
    font-size: 13px;
    user-select: text;
  }
  .search-box input::placeholder {
    color: var(--text-3);
  }
  .clear-btn {
    display: grid;
    place-items: center;
    width: 20px;
    height: 20px;
    border: 0;
    border-radius: var(--radius-xs);
    background: var(--surface-3);
    color: var(--text-2);
    cursor: pointer;
  }

  .filter-row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 6px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .chip:hover {
    background: var(--surface-2);
    border-color: var(--border-strong);
    color: var(--text);
  }
  .chip.active {
    background: var(--glow-peach);
    border-color: var(--border-glow);
    color: var(--accent-text);
  }
  .chip.subtle {
    border-style: dashed;
    background: transparent;
  }
  .chip-count {
    font-size: 11px;
    opacity: 0.78;
    font-family: var(--font-mono);
  }

  .jump-strip {
    display: none;
    gap: 6px;
    overflow-x: auto;
    padding: 2px 0 12px;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
  }
  .jump-pill {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-1);
    color: var(--text-2);
    font-family: var(--font-mono);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
  }
  .jump-pill:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .jump-pill.active {
    background: var(--glow-peach);
    border-color: var(--border-glow);
    color: var(--accent-text);
  }
  .dot-live {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--ok);
  }

  .board {
    display: grid;
    grid-template-columns: 168px minmax(0, 1fr);
    gap: 20px;
    align-items: start;
  }

  .version-rail {
    position: sticky;
    top: 8px;
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 10px;
  }
  .rail-head {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 2px 6px 8px;
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--text-3);
    border-bottom: 1px solid var(--border);
    margin-bottom: 6px;
  }
  .version-rail ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
    max-height: calc(100vh - 280px);
    overflow-y: auto;
    scrollbar-width: thin;
  }
  .rail-btn {
    width: 100%;
    display: grid;
    gap: 1px;
    padding: 7px 8px;
    border: 1px solid transparent;
    border-radius: var(--radius-s);
    background: transparent;
    text-align: left;
    cursor: pointer;
    color: var(--text-2);
  }
  .rail-btn:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .rail-btn.active {
    background: var(--glow-peach);
    border-color: var(--border-glow);
    color: var(--accent-text);
  }
  .rail-top {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
  }
  .rail-ver {
    font-family: var(--font-mono);
    font-size: 12.5px;
    font-weight: 700;
  }
  .rail-date {
    font-size: 11px;
    color: var(--text-3);
  }
  .mini-tag {
    font-size: 10px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: var(--radius-xs);
    background: var(--glow-mint);
    color: var(--ok);
  }
  .mini-tag.installed {
    background: var(--glow-lavender);
    color: var(--info);
  }

  .timeline {
    position: relative;
    display: grid;
    gap: 16px;
    padding-left: 18px;
    min-width: 0;
  }
  .timeline::before {
    content: '';
    position: absolute;
    left: 5px;
    top: 18px;
    bottom: 18px;
    width: 2px;
    background: var(--border-strong);
  }

  .release {
    position: relative;
    scroll-margin-top: 16px;
  }
  .rail-node {
    position: absolute;
    left: -18px;
    top: 22px;
    width: 12px;
    height: 12px;
    border-radius: 3px;
    background: var(--surface-2);
    border: 2px solid var(--border-strong);
  }
  .release.latest .rail-node {
    background: var(--accent);
    border-color: var(--accent);
  }

  .card {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    box-shadow: var(--shadow-card);
    overflow: hidden;
  }
  .release.latest .card {
    border-color: var(--border-glow);
  }

  .card-head {
    width: 100%;
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 16px;
    padding: 16px 18px;
    border: 0;
    background: transparent;
    text-align: left;
    cursor: pointer;
  }
  .card-head:hover {
    background: color-mix(in srgb, var(--surface-2) 55%, transparent);
  }
  .head-left {
    min-width: 0;
    flex: 1;
    display: grid;
    gap: 5px;
  }
  .ver-row {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 7px;
  }
  .ver-badge {
    font-family: var(--font-mono);
    font-size: 13px;
    font-weight: 700;
    padding: 2px 8px;
    border-radius: var(--radius-xs);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    color: var(--text);
  }
  .release.latest .ver-badge {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
  }
  .status-pill {
    font-size: 11px;
    font-weight: 700;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
  }
  .latest-pill {
    background: var(--glow-mint);
    color: var(--ok);
  }
  .installed-pill {
    background: var(--glow-lavender);
    color: var(--info);
  }
  .date {
    font-size: 12px;
    color: var(--text-3);
  }
  .codename {
    font-size: 16px;
    line-height: 1.3;
    color: var(--text);
  }
  .summary {
    margin: 0;
    font-size: 13px;
    color: var(--text-2);
    line-height: 1.5;
  }

  .head-right {
    display: flex;
    align-items: center;
    gap: 10px;
    flex: none;
  }
  .mini-counts {
    display: flex;
    flex-wrap: wrap;
    justify-content: flex-end;
    gap: 5px;
  }
  .count-pill {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    font-size: 11px;
    font-weight: 700;
    border: 1px solid var(--border);
    background: var(--surface-2);
  }
  .chev {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    border-radius: var(--radius-xs);
    color: var(--text-3);
    transition: transform var(--dur-2) var(--ease-out);
  }
  .chev.open {
    transform: rotate(180deg);
  }

  .card-body {
    padding: 0 18px 18px;
    border-top: 1px solid var(--border);
    display: grid;
    gap: 14px;
  }
  .highlights {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: 7px;
  }
  .highlights li {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 7px 10px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text);
  }
  .highlights li :global(svg) {
    color: var(--accent-text);
    flex: none;
  }

  .changes {
    list-style: none;
    margin: 14px 0 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .highlights + .changes {
    margin-top: 0;
  }
  .change-row {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 14px;
    padding: 12px 13px;
    border-radius: var(--radius-s);
    background: color-mix(in srgb, var(--surface-2) 65%, transparent);
    border: 1px solid var(--border);
  }
  .change-main {
    min-width: 0;
    flex: 1;
    display: grid;
    gap: 5px;
  }
  .change-top {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 8px;
  }
  .kind-tag {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 2px 7px;
    border-radius: var(--radius-xs);
    font-size: 11px;
    font-weight: 700;
    letter-spacing: 0.02em;
    text-transform: uppercase;
  }
  .kind-added {
    background: var(--glow-mint);
    color: var(--ok);
    border-color: color-mix(in srgb, var(--ok) 30%, transparent);
  }
  .kind-improved {
    background: var(--glow-lavender);
    color: var(--info);
    border-color: color-mix(in srgb, var(--info) 30%, transparent);
  }
  .kind-design {
    background: var(--glow-peach);
    color: var(--accent-text);
    border-color: var(--border-glow);
  }
  .kind-fixed {
    background: var(--glow-blush);
    color: var(--danger);
    border-color: color-mix(in srgb, var(--danger) 30%, transparent);
  }

  .change-title {
    font-size: 13.5px;
    font-weight: 700;
    color: var(--text);
  }
  .change-detail {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.5;
  }

  .tool-jump {
    flex: none;
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border-strong);
    background: var(--surface-1);
    color: var(--accent-text);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
  }
  .tool-jump:hover {
    background: var(--surface-3);
    border-color: var(--border-glow);
  }

  .empty-state {
    display: grid;
    justify-items: center;
    text-align: center;
    gap: 8px;
    padding: 36px 20px;
    border-radius: var(--radius-m);
    border: 1px dashed var(--border-strong);
    background: var(--surface-1);
    color: var(--text-2);
  }
  .empty-state p {
    margin: 0 0 6px;
    font-size: 13px;
    color: var(--text-3);
  }

  @media (max-width: 960px) {
    .board {
      grid-template-columns: 1fr;
    }
    .version-rail {
      display: none;
    }
    .jump-strip {
      display: flex;
    }
    .hero-right {
      align-items: flex-start;
    }
  }

  @media (max-width: 680px) {
    .timeline {
      padding-left: 0;
    }
    .timeline::before,
    .rail-node {
      display: none;
    }
    .card-head {
      flex-direction: column;
      gap: 10px;
      padding: 14px;
    }
    .head-right {
      width: 100%;
      justify-content: space-between;
    }
    .mini-counts {
      justify-content: flex-start;
    }
    .card-body {
      padding: 0 14px 14px;
    }
    .change-row {
      flex-direction: column;
      gap: 10px;
    }
  }
</style>
