<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import { formatBytes } from '../../lib/format';
  import { stIpc, type StNode } from '../../lib/ipc';
  import Tree from './Tree.svelte';
  import { storage } from './store.svelte';

  let { node, depth = 0, expandedRoot = false }: { node: StNode; depth?: number; expandedRoot?: boolean } = $props();

  // The starting value is all that matters here; later changes are the user's.
  // svelte-ignore state_referenced_locally
  let open = $state(expandedRoot);
  let kids = $state<StNode[]>([]);
  let hidden = $state(0);
  let loadedAt = -1;

  async function load() {
    const id = storage.sessionId;
    if (id == null) return;
    try {
      const page = await stIpc.children(id, node.id, 'size', true, 0, 400);
      const dirs = page.items.filter((c) => c.is_dir && !c.is_link);
      kids = dirs;
      hidden = Math.max(0, page.total - page.items.length);
      loadedAt = storage.version;
    } catch {
      kids = [];
    }
  }

  // Reload when the scan's data changed and this folder is open.
  $effect(() => {
    const v = storage.version;
    if (open && v !== loadedAt) void load();
  });

  const hasDirs = $derived(node.dirs > 0);
  const active = $derived(storage.current === node.id);
</script>

<div class="item">
  <div class="line" class:active style:padding-left="{depth * 14 + 4}px">
    {#if hasDirs}
      <button class="twist" aria-label={open ? `Collapse ${node.name}` : `Expand ${node.name}`} aria-expanded={open} onclick={() => (open = !open)}>
        {open ? '▾' : '▸'}
      </button>
    {:else}
      <span class="twist"></span>
    {/if}
    <button class="label" onclick={() => storage.go(node.id)} title={node.name}>
      <Icon name="organizer" size={14} />
      <span class="name">{node.name}</span>
      <span class="size tabular">{formatBytes(node.size)}</span>
    </button>
    <div class="bar" aria-hidden="true"><div style:width="{Math.max(1, node.percent)}%"></div></div>
  </div>
  {#if open}
    {#each kids as k (k.id)}
      <Tree node={k} depth={depth + 1} />
    {/each}
    {#if hidden > 0}<div class="more" style:padding-left="{(depth + 1) * 14 + 22}px">and {hidden} more (open the folder to see them)</div>{/if}
  {/if}
</div>

<style>
  .line { display: grid; grid-template-columns: 18px 1fr; align-items: center; position: relative; border-radius: 6px; }
  .line:hover { background: var(--surface-2); }
  .line.active { background: var(--surface-3); }
  .twist { width: 18px; border: 0; background: none; color: var(--text-3); cursor: pointer; padding: 0; font-size: 11px; }
  .label { display: flex; gap: 6px; align-items: center; border: 0; background: none; color: inherit; text-align: left; padding: 3px 4px; cursor: pointer; min-width: 0; }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1; }
  .size { color: var(--text-3); font-size: 12px; flex: none; }
  .bar { grid-column: 2; height: 2px; background: transparent; margin: 0 4px 2px; }
  .bar div { height: 100%; background: var(--accent); opacity: 0.6; border-radius: 1px; }
  .more { color: var(--text-3); font-size: 12px; padding: 2px 0; }
</style>
