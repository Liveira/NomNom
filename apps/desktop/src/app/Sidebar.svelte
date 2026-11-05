<script lang="ts">
  import Icon from '../lib/icons/Icon.svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import { FOOTER_MODULES } from '../lib/modules';
  import { app } from './state.svelte';

  const collapsed = $derived(app.settings?.appearance.sidebar_collapsed ?? false);
  const main = $derived(app.nav.filter((n) => !FOOTER_MODULES.has(n.id)));
  const footer = $derived(app.nav.filter((n) => FOOTER_MODULES.has(n.id)));
</script>

<nav class="sidebar" class:collapsed aria-label="Main">
  <div class="brand">
    <Logo size={28} />
    {#if !collapsed}<span class="wordmark">NOMNOM</span>{/if}
  </div>

  <ul>
    {#each main as item (item.id)}
      <li>
        <button
          class="item"
          class:active={app.current === item.id}
          aria-current={app.current === item.id ? 'page' : undefined}
          title={collapsed ? item.name : undefined}
          onclick={() => app.go(item.id)}
        >
          <Icon name={item.icon} />
          {#if !collapsed}<span>{item.name}</span>{/if}
        </button>
      </li>
    {/each}
  </ul>

  <div class="spacer"></div>

  <ul>
    {#each footer as item (item.id)}
      <li>
        <button
          class="item"
          class:active={app.current === item.id}
          aria-current={app.current === item.id ? 'page' : undefined}
          title={collapsed ? item.name : undefined}
          onclick={() => app.go(item.id)}
        >
          <Icon name={item.icon} />
          {#if !collapsed}<span>{item.name}</span>{/if}
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
        <Icon name="panel" />
        {#if !collapsed}<span>Collapse</span>{/if}
      </button>
    </li>
  </ul>
</nav>

<style>
  .sidebar {
    width: 208px;
    flex: none;
    display: flex;
    flex-direction: column;
    padding: 14px 10px;
    background: var(--surface-1);
    border-right: 1px solid var(--border);
    transition: width var(--dur-3) var(--ease-out);
  }
  .sidebar.collapsed { width: 64px; }
  .brand { display: flex; align-items: center; gap: 10px; padding: 4px 8px 18px; height: 44px; }
  .wordmark { font-family: var(--font-display); font-weight: 700; letter-spacing: 0.04em; font-size: 15px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .spacer { flex: 1; }
  .item {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border: 0;
    border-radius: var(--radius-s);
    background: transparent;
    color: var(--text-2);
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
    transition: background var(--dur-1), color var(--dur-1);
  }
  .item:hover { background: var(--surface-2); color: var(--text); }
  .item.active { background: var(--surface-3); color: var(--text); }
  .item.active :global(svg) { color: var(--accent); }
  .collapsed .item { justify-content: center; padding-inline: 0; }
</style>
