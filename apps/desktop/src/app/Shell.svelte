<script lang="ts">
  import type { Component } from 'svelte';
  import Backdrop from '../lib/fx/Backdrop.svelte';
  import Sidebar from './Sidebar.svelte';
  import Palette from './Palette.svelte';
  import TitleBar from './TitleBar.svelte';
  import { app } from './state.svelte';

  let View = $state<Component | null>(null);
  let main = $state<HTMLElement | null>(null);
  let loadError = $state<string | null>(null);

  $effect(() => {
    const item = app.nav.find((n) => n.id === app.current);
    if (!item) return;
    // A new page starts at the top.
    if (main) main.scrollTop = 0;
    let stale = false;
    item.view().then(
      (m) => {
        if (!stale) {
          View = m.default;
          loadError = null;
        }
      },
      (e) => {
        if (!stale) loadError = String(e);
      },
    );
    return () => {
      stale = true;
    };
  });

  function keys(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && !e.altKey && !e.shiftKey && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      app.paletteOpen = !app.paletteOpen;
    }
  }
</script>

<svelte:window onkeydown={keys} />
<Backdrop theme={app.palette} motion={app.motion} />
<TitleBar />
<div class="shell">
  <Sidebar />
  <main bind:this={main}>
    {#key app.current}
      <div class="view">
        {#if loadError}
          <p class="error">Couldn't open this section. {loadError}</p>
        {:else if View}
          <View />
        {/if}
      </div>
    {/key}
  </main>
</div>
{#if app.paletteOpen}
  <Palette onclose={() => (app.paletteOpen = false)} />
{/if}

<style>
  .shell {
    display: flex;
    height: calc(100% - var(--titlebar-h));
    gap: 4px;
    position: relative;
    z-index: 1;
  }
  main {
    flex: 1;
    min-width: 0;
    overflow: auto;
  }
  .view {
    padding: 22px 28px 34px;
    max-width: 1280px;
    animation: enter var(--dur-3) var(--ease-out);
  }
  .error { color: var(--danger); }
  @keyframes enter {
    from { opacity: 0; transform: translateY(7px); }
    to { opacity: 1; transform: none; }
  }
</style>
