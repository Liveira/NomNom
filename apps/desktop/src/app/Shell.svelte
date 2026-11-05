<script lang="ts">
  import type { Component } from 'svelte';
  import Sidebar from './Sidebar.svelte';
  import { app } from './state.svelte';

  let View = $state<Component | null>(null);
  let loadError = $state<string | null>(null);

  // Load the active module's view lazily whenever the selection changes.
  $effect(() => {
    const item = app.nav.find((n) => n.id === app.current);
    if (!item) return;
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
</script>

<div class="shell">
  <Sidebar />
  <main>
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

<style>
  .shell { display: flex; height: 100%; }
  main { flex: 1; min-width: 0; overflow: auto; }
  .view { padding: 28px 32px; animation: enter var(--dur-3) var(--ease-out); }
  .error { color: var(--danger); }
  @keyframes enter {
    from { opacity: 0; transform: translateY(6px); }
    to { opacity: 1; transform: none; }
  }
</style>
