<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import ProgressBar from '../lib/ui/ProgressBar.svelte';
  import { onUpdateProgress, updateIpc } from '../lib/ipc';
  import { FOOTER_MODULES } from '../lib/modules';
  import { app } from './state.svelte';

  const collapsed = $derived(app.settings?.appearance.sidebar_collapsed ?? false);
  const main = $derived(app.nav.filter((n) => !FOOTER_MODULES.has(n.id)));
  const footer = $derived(app.nav.filter((n) => FOOTER_MODULES.has(n.id)));
  const playActive = $derived(app.settings?.activity.enabled ?? true);

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

<nav class="sidebar" class:collapsed aria-label="Main">
  <button class="brand" onclick={() => app.go('home')} title="NomNom Home">
    <div class="logo-orb">
      <Logo size={28} />
    </div>
    {#if !collapsed}
      <div class="brand-text">
        <span class="wordmark">NomNom</span>
        <span class="submark">
          <span class="status-dot" class:on={playActive}></span>
          {playActive ? 'tracking active' : 'studio hub'}
        </span>
      </div>
    {/if}
  </button>

  <ul class="nav-list">
    {#each main as item (item.id)}
      <li>
        <button
          class="item"
          class:active={app.current === item.id}
          aria-current={app.current === item.id ? 'page' : undefined}
          title={collapsed ? item.name : undefined}
          onclick={() => app.go(item.id)}
        >
          <span class="icon-wrap"><Icon name={item.icon} size={19} /></span>
          {#if !collapsed}<span class="label">{item.name}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>

  <div class="spacer"></div>

  {#if app.newVersion && !collapsed}
    <div class="update-card" role="status">
      <div class="u-top">
        <Icon name="update" size={15} />
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

  <ul class="nav-list footer-list">
    {#each footer as item (item.id)}
      <li>
        <button
          class="item"
          class:active={app.current === item.id}
          aria-current={app.current === item.id ? 'page' : undefined}
          title={collapsed ? item.name : undefined}
          onclick={() => app.go(item.id)}
        >
          <span class="icon-wrap"><Icon name={item.icon} size={19} /></span>
          {#if !collapsed}<span class="label">{item.name}</span>{/if}
          {#if item.id === 'settings' && app.newVersion}
            <span class="dot" role="img" aria-label="Update available" title="Update available"></span>
          {/if}
        </button>
      </li>
    {/each}
    <li>
      <button
        class="item"
        aria-label={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
        onclick={() => app.toggleSidebar()}
      >
        <span class="icon-wrap"><Icon name="panel" size={19} /></span>
        {#if !collapsed}<span class="label">Collapse</span>{/if}
      </button>
    </li>
  </ul>
</nav>

<style>
  .sidebar {
    width: 224px;
    flex: none;
    display: flex;
    flex-direction: column;
    margin: 12px 0 12px 12px;
    padding: 14px 11px;
    background: linear-gradient(180deg, var(--surface-1) 0%, rgba(26, 20, 36, 0.92) 100%);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-l);
    box-shadow: var(--shadow-card);
    transition: width var(--dur-3) var(--ease-out);
  }
  :global(:root[data-theme='light']) .sidebar {
    background: var(--surface-1);
  }
  .sidebar.collapsed {
    width: 68px;
    padding-inline: 9px;
  }

  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 6px 7px 14px;
    margin-bottom: 6px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    cursor: pointer;
    text-align: left;
  }
  .collapsed .brand {
    justify-content: center;
    padding-inline: 0;
  }
  .logo-orb {
    width: 38px;
    height: 38px;
    border-radius: 13px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-soft);
    flex: none;
  }
  .brand-text {
    display: grid;
    gap: 1px;
    min-width: 0;
  }
  .wordmark {
    font-family: var(--font-display);
    font-weight: 700;
    letter-spacing: 0.01em;
    font-size: 17px;
    line-height: 1.1;
  }
  .submark {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    color: var(--text-3);
    white-space: nowrap;
  }
  .status-dot {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--text-3);
  }
  .status-dot.on {
    background: var(--ok);
    box-shadow: 0 0 7px var(--ok);
  }

  .nav-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 4px;
  }
  .footer-list {
    padding-top: 8px;
    border-top: 1px solid var(--border);
  }
  .spacer { flex: 1; }

  .item {
    position: relative;
    width: 100%;
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 8px 11px;
    border: 1px solid transparent;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    font-weight: 500;
    transition: background var(--dur-1), color var(--dur-1), border-color var(--dur-1);
  }
  .icon-wrap {
    width: 24px;
    height: 24px;
    display: grid;
    place-items: center;
    border-radius: 8px;
    flex: none;
    transition: transform var(--dur-2) var(--ease-spring);
  }
  .item:hover {
    background: var(--surface-2);
    color: var(--text);
  }
  .item:hover .icon-wrap {
    transform: scale(1.07);
  }
  .item.active {
    background: linear-gradient(135deg, var(--surface-3), var(--surface-2));
    border-color: var(--border-strong);
    color: var(--text);
    font-weight: 700;
    box-shadow: var(--shadow-soft);
  }
  .item.active :global(svg) {
    color: var(--accent);
  }
  .collapsed .item {
    justify-content: center;
    padding-inline: 0;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    margin-left: auto;
  }
  .collapsed .dot {
    position: absolute;
    top: 6px;
    right: 10px;
    margin: 0;
  }

  .update-card {
    margin-bottom: 10px;
    padding: 10px 12px;
    border-radius: var(--radius-m);
    background:
      radial-gradient(120px 80px at 100% 0%, var(--glow-peach), transparent 75%),
      var(--surface-2);
    border: 1px solid var(--border-glow);
    display: grid;
    gap: 7px;
  }
  .u-top {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12.5px;
    color: var(--accent-text);
  }
  .u-btn {
    border: 0;
    padding: 6px 10px;
    border-radius: var(--radius-xs);
    background: linear-gradient(135deg, var(--accent), var(--blush));
    color: var(--on-accent);
    font-weight: 700;
    font-size: 12.5px;
    cursor: pointer;
  }
  .u-sub {
    font-size: 11px;
    color: var(--text-3);
  }
</style>
