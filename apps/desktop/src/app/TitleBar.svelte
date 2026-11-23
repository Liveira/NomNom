<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from '../lib/icons/Icon.svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import { app } from './state.svelte';

  let maximized = $state(false);
  const title = $derived(app.nav.find((n) => n.id === app.current)?.name ?? 'NomNom');
  const searchKey = $derived(app.settings?.shortcuts.launcher || 'Ctrl+K');

  function win() {
    try {
      return getCurrentWindow();
    } catch {
      return null;
    }
  }

  $effect(() => {
    const w = win();
    if (!w) return;
    let off: (() => void) | undefined;
    const sync = () => void w.isMaximized().then((m) => (maximized = m)).catch(() => {});
    sync();
    void w.onResized(sync).then((u) => (off = u)).catch(() => {});
    return () => off?.();
  });

  function onKey(e: KeyboardEvent) {
    if (!e.altKey) return;
    if (e.key === 'ArrowLeft') {
      e.preventDefault();
      app.back();
    } else if (e.key === 'ArrowRight') {
      e.preventDefault();
      app.forward();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<header class="titlebar app-titlebar" data-tauri-drag-region>
  <div class="side left" data-tauri-drag-region>
    <button class="nav" aria-label="Back" title="Back (Alt+Left)" disabled={!app.canGoBack} onclick={() => app.back()}>
      <Icon name="arrow-left" size={16} />
    </button>
    <button class="nav" aria-label="Forward" title="Forward (Alt+Right)" disabled={!app.canGoForward} onclick={() => app.forward()}>
      <Icon name="arrow" size={16} />
    </button>
    <div class="crumb" data-tauri-drag-region>
      <Logo size={16} />
      <span data-tauri-drag-region>{title}</span>
    </div>
  </div>

  <button class="search-trigger" aria-label="Search" title={`Search (${searchKey})`} onclick={() => (app.paletteOpen = true)}>
    <Icon name="search" size={14} />
    <span>Search</span>
    <kbd>{searchKey}</kbd>
  </button>

  <div class="side right" data-tauri-drag-region>
    <button class="ctl" aria-label="Minimize" title="Minimize" onclick={() => void win()?.minimize()}>
      <Icon name="minus" size={15} />
    </button>
    <button
      class="ctl"
      aria-label={maximized ? 'Restore' : 'Maximize'}
      title={maximized ? 'Restore' : 'Maximize'}
      onclick={() => void win()?.toggleMaximize()}
    >
      <Icon name={maximized ? 'copy' : 'square'} size={maximized ? 13 : 12} />
    </button>
    <button class="ctl close" aria-label="Close" title="Close" onclick={() => void win()?.close()}>
      <Icon name="x" size={16} />
    </button>
  </div>
</header>

<style>
  .titlebar {
    height: var(--titlebar-h);
    display: grid;
    grid-template-columns: 1fr auto 1fr;
    align-items: center;
    position: relative;
    z-index: 3;
    user-select: none;
    -webkit-user-select: none;
    background: var(--surface-1);
    border-bottom: 1px solid var(--border);
  }
  .side {
    display: flex;
    align-items: center;
    height: 100%;
    min-width: 0;
  }
  .left { padding-left: 10px; gap: 3px; }
  .right { justify-content: flex-end; }
  .crumb {
    display: inline-flex;
    align-items: center;
    gap: 7px;
    margin-left: 8px;
    font-size: 12.5px;
    font-weight: 600;
    color: var(--text-2);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .search-trigger {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    height: 26px;
    min-width: 210px;
    padding: 0 8px 0 11px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--bg);
    color: var(--text-3);
    font-size: 12px;
    cursor: pointer;
  }
  .search-trigger:hover {
    border-color: var(--border-strong);
    color: var(--text-2);
  }
  .search-trigger kbd {
    margin-left: auto;
    font: 600 10.5px var(--font-mono);
    padding: 1px 5px;
    border-radius: 4px;
    background: var(--surface-2);
    color: var(--text-3);
  }
  button {
    border: 0;
    background: none;
    color: var(--text-2);
    cursor: pointer;
    display: grid;
    place-items: center;
  }
  .nav {
    width: 26px;
    height: 26px;
    border-radius: var(--radius-xs);
  }
  .nav:hover:not(:disabled), .ctl:hover { background: var(--surface-2); color: var(--text); }
  .nav:disabled { opacity: 0.32; cursor: default; }
  .ctl {
    width: 44px;
    height: 100%;
  }
  .ctl.close:hover { background: #e5384b; color: #fff; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  @media (max-width: 620px) {
    .search-trigger { min-width: auto; }
    .search-trigger span, .search-trigger kbd { display: none; }
  }
</style>
