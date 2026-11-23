<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import ProgressBar from '../lib/ui/ProgressBar.svelte';
  import { onUpdateProgress, updateIpc } from '../lib/ipc';
  import { FOOTER_MODULES } from '../lib/modules';
  import { app } from './state.svelte';

  const collapsed = $derived(app.settings?.appearance.sidebar_collapsed ?? false);
  const main = $derived(app.sidebarNav.filter((n) => !FOOTER_MODULES.has(n.id)));
  const footer = $derived(app.sidebarNav.filter((n) => FOOTER_MODULES.has(n.id)));

  let updating = $state(false);
  let updatePct = $state<number | null>(null);
  let updateMsg = $state('');
  let stopProgress: (() => void) | null = null;

  void onUpdateProgress((p) => {
    updatePct = p.total && p.total > 0 ? Math.min(1, p.done / p.total) : null;
  }).then((u) => (stopProgress = u));
  onDestroy(() => stopProgress?.());

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

<nav class="sidebar app-sidebar" class:collapsed aria-label="Main">
  <button class="brand app-brand" onclick={() => app.go('home')} title="NomNom Home">
    <Logo size={24} />
    {#if !collapsed}<span class="wordmark">NomNom</span>{/if}
  </button>

  <ul class="nav-list app-nav-list">
    {#each main as item (item.id)}
      <li>
        <button
          class="item app-nav-item"
          class:active={app.current === item.id}
          aria-current={app.current === item.id ? 'page' : undefined}
          title={collapsed ? item.name : undefined}
          onclick={() => app.go(item.id)}
        >
          <span class="icon-wrap"><Icon name={item.icon} size={18} /></span>
          {#if !collapsed}<span class="label">{item.name}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>

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
          title={collapsed ? item.name : undefined}
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
        onclick={() => app.toggleSidebar()}
      >
        <span class="icon-wrap"><Icon name="panel" size={18} /></span>
        {#if !collapsed}<span class="label">Collapse</span>{/if}
      </button>
    </li>
  </ul>
</nav>

<style>
  .sidebar {
    width: 214px;
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 12px 10px 10px;
    background: var(--surface-1);
    border-right: 1px solid var(--border);
    transition: width var(--dur-2) var(--ease-out);
    position: relative;
    z-index: 2;
  }
  .sidebar.collapsed {
    width: 56px;
    padding-inline: 8px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 4px 8px 14px;
    margin-bottom: 4px;
    border: 0;
    background: none;
    cursor: pointer;
    text-align: left;
  }
  .collapsed .brand {
    justify-content: center;
    padding-inline: 0;
  }
  .wordmark {
    font-family: var(--font-display);
    font-weight: 700;
    letter-spacing: -0.01em;
    font-size: 16px;
    line-height: 1;
  }

  .nav-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 2px;
  }
  .footer-list {
    padding-top: 8px;
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
  .nav-list:not(.footer-list)::-webkit-scrollbar-thumb { background: var(--border-strong); border-radius: 4px; }

  .spacer { flex: 1; min-height: 8px; }

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
  }
  .collapsed .dot {
    position: absolute;
    top: 6px;
    right: 8px;
    margin: 0;
  }

  .update-card {
    margin-bottom: 8px;
    padding: 10px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    display: grid;
    gap: 7px;
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
</style>
