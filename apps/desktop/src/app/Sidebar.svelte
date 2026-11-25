<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import ProgressBar from '../lib/ui/ProgressBar.svelte';
  import { onUpdateProgress, updateIpc } from '../lib/ipc';
  import { FOOTER_MODULES, type NavItem } from '../lib/modules';
  import { themeLayout } from '../lib/themes';
  import { CORE, TOOL_CATEGORIES, toolCategoryOf } from '../lib/tools';
  import { app } from './state.svelte';

  let winW = $state(typeof window !== 'undefined' ? window.innerWidth : 1040);
  let navEl = $state<HTMLUListElement | null>(null);
  let canScrollLeft = $state(false);
  let canScrollRight = $state(false);

  const layout = $derived(themeLayout(app.palette));
  const isNarrow = $derived(winW <= 920);
  const supportsDrawer = $derived(layout !== 'dock-top' && layout !== 'rail');
  const drawerOpen = $derived(isNarrow && supportsDrawer && app.navDrawerOpen);

  /**
   * Responsive collapse rules:
   * - `dock-top` and `rail` manage their own structural geometry.
   * - On narrow windows (<= 920px), vertical sidebars auto-compact to an icon bar unless
   *   the slide-over drawer is open.
   * - On desktop (> 920px), the saved `sidebar_collapsed` preference applies.
   */
  const collapsed = $derived(
    !supportsDrawer
      ? false
      : isNarrow
        ? !drawerOpen
        : (app.settings?.appearance.sidebar_collapsed ?? false),
  );

  // Close the overlay drawer automatically whenever the window widens past the narrow threshold
  // or switches to a layout that doesn't use a vertical drawer.
  $effect(() => {
    if ((!isNarrow || !supportsDrawer) && app.navDrawerOpen) {
      app.navDrawerOpen = false;
    }
  });

  const main = $derived(app.sidebarNav.filter((n) => !FOOTER_MODULES.has(n.id)));
  const footer = $derived(app.sidebarNav.filter((n) => FOOTER_MODULES.has(n.id)));
  const coreNav = $derived(main.filter((n) => CORE.has(n.id)));
  const toolNav = $derived(main.filter((n) => !CORE.has(n.id)));
  const showCategoryGroups = $derived(!collapsed && supportsDrawer && toolNav.length >= 6);
  const groupedTools = $derived.by(() => {
    const groups: { id: string; title: string; items: NavItem[] }[] = [];
    for (const cat of TOOL_CATEGORIES) {
      const items = toolNav.filter((n) => toolCategoryOf(n.id)?.id === cat.id);
      if (items.length > 0) groups.push({ id: cat.id, title: cat.title, items });
    }
    const uncategorized = toolNav.filter((n) => !toolCategoryOf(n.id));
    if (uncategorized.length > 0) groups.push({ id: 'other', title: 'Other', items: uncategorized });
    return groups;
  });

  function syncHorizontalScroll() {
    if (!navEl || layout !== 'dock-top') {
      canScrollLeft = false;
      canScrollRight = false;
      return;
    }
    const max = navEl.scrollWidth - navEl.clientWidth;
    canScrollLeft = navEl.scrollLeft > 4;
    canScrollRight = max - navEl.scrollLeft > 4;
  }

  $effect(() => {
    void layout;
    void main.length;
    void winW;
    const el = navEl;
    if (!el || layout !== 'dock-top') {
      canScrollLeft = false;
      canScrollRight = false;
      return;
    }
    syncHorizontalScroll();
    const ro = typeof ResizeObserver !== 'undefined' ? new ResizeObserver(syncHorizontalScroll) : null;
    ro?.observe(el);
    return () => ro?.disconnect();
  });

  function scrollDock(dir: -1 | 1) {
    if (!navEl) return;
    navEl.scrollBy({ left: dir * Math.max(160, Math.round(navEl.clientWidth * 0.6)), behavior: 'smooth' });
  }

  function onNavWheel(e: WheelEvent) {
    if (layout !== 'dock-top' || Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
    const el = e.currentTarget as HTMLElement;
    if (el.scrollWidth > el.clientWidth) {
      e.preventDefault();
      el.scrollLeft += e.deltaY;
      syncHorizontalScroll();
    }
  }

  function onToggleClick() {
    if (isNarrow && supportsDrawer) {
      app.navDrawerOpen = !app.navDrawerOpen;
    } else {
      void app.toggleSidebar();
    }
  }

  let updating = $state(false);
  let updatePct = $state<number | null>(null);
  let updateMsg = $state('');
  let stopProgress: (() => void) | null = null;
  let gone = false;

  void onUpdateProgress((p) => {
    if (!gone) updatePct = p.total && p.total > 0 ? Math.min(1, p.done / p.total) : null;
  })
    .then((u) => {
      if (gone) u();
      else stopProgress = u;
    })
    .catch(() => {});

  onDestroy(() => {
    gone = true;
    stopProgress?.();
  });

  async function installUpdateNow() {
    if (updating) return;
    updating = true;
    updatePct = null;
    updateMsg = '';
    try {
      const msg = await updateIpc.install();
      if (msg) {
        updateMsg = msg;
        app.newVersion = null;
      }
    } catch (e) {
      updateMsg = String(e);
    } finally {
      updating = false;
    }
  }
</script>

<svelte:window bind:innerWidth={winW} />

{#if drawerOpen}
  <div class="rail-ghost app-rail-ghost" aria-hidden="true"></div>
  <button
    class="drawer-backdrop"
    aria-label="Close menu"
    onclick={() => (app.navDrawerOpen = false)}
  ></button>
{/if}

<nav
  class="sidebar app-sidebar"
  class:collapsed
  class:drawer-open={drawerOpen}
  class:narrow={isNarrow}
  aria-label="Main"
>
  <div class="brand-row app-brand-row">
    <button class="brand app-brand" onclick={() => app.go('home')} title="NomNom Home">
      <Logo size={24} />
      {#if !collapsed}<span class="wordmark">NomNom</span>{/if}
    </button>
    {#if drawerOpen}
      <button class="drawer-close" aria-label="Close menu" title="Close menu" onclick={() => (app.navDrawerOpen = false)}>
        <Icon name="x" size={16} />
      </button>
    {/if}
  </div>

  {#if layout === 'dock-top' && canScrollLeft}
    <button class="dock-scroll" aria-label="Scroll menu left" title="Scroll menu left" onclick={() => scrollDock(-1)}>
      <Icon name="chevron-left" size={15} />
    </button>
  {/if}

  <ul
    class="nav-list app-nav-list"
    class:fade-left={layout === 'dock-top' && canScrollLeft}
    class:fade-right={layout === 'dock-top' && canScrollRight}
    bind:this={navEl}
    onwheel={onNavWheel}
    onscroll={syncHorizontalScroll}
  >
    {#if showCategoryGroups}
      {#each coreNav as item (item.id)}
        <li>
          <button
            class="item app-nav-item"
            class:active={app.current === item.id}
            aria-current={app.current === item.id ? 'page' : undefined}
            title={item.name}
            onclick={() => app.go(item.id)}
          >
            <span class="icon-wrap"><Icon name={item.icon} size={18} /></span>
            <span class="label">{item.name}</span>
          </button>
        </li>
      {/each}
      {#each groupedTools as group (group.id)}
        <li class="nav-group-label app-nav-group-label" aria-hidden="true">{group.title}</li>
        {#each group.items as item (item.id)}
          <li>
            <button
              class="item app-nav-item"
              class:active={app.current === item.id}
              aria-current={app.current === item.id ? 'page' : undefined}
              title={item.name}
              onclick={() => app.go(item.id)}
            >
              <span class="icon-wrap"><Icon name={item.icon} size={18} /></span>
              <span class="label">{item.name}</span>
            </button>
          </li>
        {/each}
      {/each}
    {:else}
      {#each main as item (item.id)}
        <li>
          <button
            class="item app-nav-item"
            class:active={app.current === item.id}
            aria-current={app.current === item.id ? 'page' : undefined}
            title={item.name}
            onclick={() => app.go(item.id)}
          >
            <span class="icon-wrap"><Icon name={item.icon} size={18} /></span>
            {#if !collapsed}<span class="label">{item.name}</span>{/if}
          </button>
        </li>
      {/each}
    {/if}
  </ul>

  {#if layout === 'dock-top' && canScrollRight}
    <button class="dock-scroll" aria-label="Scroll menu right" title="Scroll menu right" onclick={() => scrollDock(1)}>
      <Icon name="chevron-right" size={15} />
    </button>
  {/if}

  <div class="spacer app-spacer"></div>

  {#if app.newVersion && !collapsed}
    <div class="update-card" role="status">
      <div class="u-top">
        <Icon name="update" size={14} />
        <strong>NomNom {app.newVersion.version}</strong>
      </div>
      {#if updating}
        <ProgressBar value={updatePct} label="Downloading update" />
        <span class="u-sub">Updating in place…</span>
      {:else}
        <button class="u-btn" onclick={installUpdateNow}>Update now</button>
      {/if}
      {#if updateMsg}<span class="u-sub">{updateMsg}</span>{/if}
    </div>
  {/if}

  <ul class="nav-list footer-list app-nav-footer">
    {#each footer as item (item.id)}
      <li>
        <button
          class="item app-nav-item"
          class:active={app.current === item.id}
          aria-current={app.current === item.id ? 'page' : undefined}
          title={item.name}
          onclick={() => app.go(item.id)}
        >
          <span class="icon-wrap"><Icon name={item.icon} size={18} /></span>
          {#if !collapsed}<span class="label">{item.name}</span>{/if}
          {#if item.id === 'settings' && app.newVersion}
            <span class="dot" role="img" aria-label="Update available" title="Update available"></span>
          {/if}
        </button>
      </li>
    {/each}
    <li class="app-collapse-item">
      <button
        class="item app-nav-item"
        aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        onclick={onToggleClick}
      >
        <span class="icon-wrap"><Icon name="panel" size={18} /></span>
        {#if !collapsed}<span class="label">Collapse</span>{/if}
      </button>
    </li>
  </ul>
</nav>

<style>
  .rail-ghost {
    width: 56px;
    flex: none;
  }
  .drawer-backdrop {
    position: absolute;
    inset: 0;
    z-index: 20;
    border: 0;
    padding: 0;
    background: rgba(0, 0, 0, 0.38);
    backdrop-filter: blur(2px);
    cursor: default;
  }

  .sidebar {
    width: clamp(196px, 16.5vw, 232px);
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 10px 9px 9px;
    background: var(--surface-1);
    border-right: 1px solid var(--border);
    transition: width var(--dur-2) var(--ease-out);
    position: relative;
    z-index: 2;
    min-height: 0;
  }
  .sidebar.collapsed {
    width: 56px;
    padding-inline: 7px;
  }
  .sidebar.drawer-open {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: min(252px, calc(100vw - 48px));
    z-index: 25;
    box-shadow: var(--shadow-pop);
  }

  .brand-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 6px;
    margin-bottom: 4px;
    flex: none;
  }
  .collapsed .brand-row {
    justify-content: center;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 10px;
    border: 0;
    background: none;
    cursor: pointer;
    text-align: left;
    min-width: 0;
  }
  .collapsed .brand {
    justify-content: center;
    padding: 4px 0 10px;
  }
  .wordmark {
    font-family: var(--font-display);
    font-weight: 700;
    letter-spacing: -0.01em;
    font-size: 16px;
    line-height: 1;
    white-space: nowrap;
  }
  .drawer-close {
    display: grid;
    place-items: center;
    width: 28px;
    height: 28px;
    margin-bottom: 6px;
    border: 1px solid var(--border);
    border-radius: var(--radius-xs);
    background: var(--surface-2);
    color: var(--text-2);
    cursor: pointer;
    flex: none;
  }
  .drawer-close:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .dock-scroll {
    flex: none;
    width: 26px;
    height: 28px;
    display: grid;
    place-items: center;
    border: 1px solid var(--border);
    border-radius: var(--radius-xs);
    background: var(--surface-2);
    color: var(--text-2);
    cursor: pointer;
  }
  .dock-scroll:hover {
    background: var(--surface-3);
    color: var(--text);
  }

  .nav-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .footer-list {
    padding-top: 7px;
    border-top: 1px solid var(--border);
    flex: none;
  }
  .nav-list:not(.footer-list) {
    flex: 0 1 auto;
    min-height: 0;
    overflow-y: auto;
    overflow-x: hidden;
    scrollbar-width: thin;
    scrollbar-color: var(--border-strong) transparent;
    overscroll-behavior: contain;
  }
  .nav-list:not(.footer-list)::-webkit-scrollbar { width: 4px; }
  .nav-list:not(.footer-list)::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 3px; }

  .spacer { flex: 1; min-height: 6px; }
  .nav-group-label {
    padding: 9px 10px 3px;
    font-size: 10px;
    font-weight: 700;
    letter-spacing: 0.07em;
    text-transform: uppercase;
    color: var(--text-3);
    user-select: none;
  }

  .item {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 10px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    font-size: 13px;
    font-weight: 500;
  }
  .icon-wrap {
    width: 20px;
    height: 20px;
    display: grid;
    place-items: center;
    flex: none;
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    min-width: 0;
  }
  .item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .item.active {
    background: var(--surface-2);
    color: var(--text);
    font-weight: 700;
  }
  .item.active :global(svg) {
    color: var(--accent);
  }
  .item.active::before {
    content: '';
    position: absolute;
    left: 0;
    top: 22%;
    bottom: 22%;
    width: 2.5px;
    border-radius: 2px;
    background: var(--accent);
  }
  .collapsed .item {
    justify-content: center;
    padding-inline: 0;
  }
  .collapsed .item.active::before {
    display: none;
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--accent);
    margin-left: auto;
    flex: none;
  }
  .collapsed .dot {
    position: absolute;
    top: 6px;
    right: 8px;
    margin: 0;
  }

  .update-card {
    margin-bottom: 8px;
    padding: 9px 10px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    display: grid;
    gap: 6px;
    flex: none;
  }
  .u-top {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--accent-text);
  }
  .u-btn {
    border: 0;
    padding: 6px 10px;
    border-radius: var(--radius-xs);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: 700;
    font-size: 12px;
    cursor: pointer;
  }
  .u-btn:hover { background: var(--accent-hover); }
  .u-sub { font-size: 11px; color: var(--text-3); }

  /* Compact vertical rhythm on shorter screens (520px - 660px tall) */
  @media (max-height: 660px) {
    .sidebar {
      padding: 7px 8px 6px;
    }
    .brand {
      padding: 2px 8px 6px;
    }
    .nav-group-label {
      padding: 6px 10px 2px;
    }
    .item {
      padding: 5px 9px;
      font-size: 12.5px;
    }
    .footer-list {
      padding-top: 5px;
    }
    .update-card {
      padding: 7px 8px;
      margin-bottom: 5px;
    }
  }
</style>
