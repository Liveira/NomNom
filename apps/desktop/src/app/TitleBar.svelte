<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import Icon from '../lib/icons/Icon.svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import { app } from './state.svelte';

  let maximized = $state(false);
  const title = $derived(app.nav.find((n) => n.id === app.current)?.name ?? 'NomNom');

  // The native frame is hidden, so this bar draws the window buttons itself.
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

<header class="titlebar" data-tauri-drag-region>
  <div class="side left" data-tauri-drag-region>
    <button class="nav" aria-label="Back" title="Back (Alt+Left)" disabled={!app.canGoBack} onclick={() => app.back()}>
      <Icon name="arrow-left" size={17} />
    </button>
    <button class="nav" aria-label="Forward" title="Forward (Alt+Right)" disabled={!app.canGoForward} onclick={() => app.forward()}>
      <Icon name="arrow" size={17} />
    </button>
  </div>

  <div class="title" data-tauri-drag-region>
    <Logo size={17} />
    <span data-tauri-drag-region>{title}</span>
  </div>

  <div class="side right">
    <button class="ctl" aria-label="Minimize" title="Minimize" onclick={() => void win()?.minimize()}>
      <Icon name="minus" size={16} />
    </button>
    <button
      class="ctl"
      aria-label={maximized ? 'Restore' : 'Maximize'}
      title={maximized ? 'Restore' : 'Maximize'}
      onclick={() => void win()?.toggleMaximize()}
    >
      <Icon name={maximized ? 'copy' : 'square'} size={maximized ? 14 : 13} />
    </button>
    <button class="ctl close" aria-label="Close" title="Close" onclick={() => void win()?.close()}>
      <Icon name="x" size={17} />
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
    /* See-through, so the animated background of the theme runs under the bar. */
    background: color-mix(in srgb, var(--surface-1) 84%, transparent);
    border-bottom: 1px solid var(--border);
  }
  .side {
    display: flex;
    align-items: center;
    height: 100%;
  }
  .left { padding-left: 10px; gap: 2px; }
  .right { justify-content: flex-end; }
  .title {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-display);
    font-weight: 600;
    font-size: 0.86rem;
    color: var(--text);
  }
  button {
    border: 0;
    background: none;
    color: var(--text-2);
    cursor: pointer;
    display: grid;
    place-items: center;
    transition: background var(--dur-1), color var(--dur-1);
  }
  .nav {
    width: 30px;
    height: 26px;
    border-radius: var(--radius-xs);
  }
  .nav:hover:not(:disabled), .ctl:hover { background: color-mix(in srgb, var(--text) 12%, transparent); color: var(--text); }
  .nav:disabled { opacity: 0.38; cursor: default; }
  .ctl {
    width: 46px;
    height: 100%;
  }
  .ctl.close:hover { background: #e5384b; color: #fff; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
</style>
