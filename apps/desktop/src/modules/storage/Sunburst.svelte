<script lang="ts">
  import { stIpc, type StTree } from '../../lib/ipc';
  import { formatUnitBytes } from './logic';
  import { sunburstArcs, type RingArc } from './charts';
  import { AGE_PALETTE, CATEGORY_COLORS, CATEGORY_LABELS, resolveColor, type ColorMode, type Metric } from './treemap';
  import { storage } from './store.svelte';

  let { oncontext }: { oncontext?: (id: number, x: number, y: number) => void } = $props();

  let tree = $state<StTree | null>(null);
  let metric = $state<Metric>('size');
  let colorMode = $state<ColorMode>('folder');
  let depth = $state(4);
  let hover = $state<RingArc | null>(null);
  let error = $state('');
  let seq = 0;

  async function load() {
    const id = storage.sessionId;
    if (id == null) return;
    const mine = ++seq;
    try {
      const t = await stIpc.treemap(id, storage.current, depth, 140);
      if (mine === seq) {
        tree = t;
        error = '';
      }
    } catch (e) {
      if (mine === seq) error = String(e);
    }
  }

  $effect(() => {
    void [storage.sessionId, storage.current, storage.version, depth];
    void load();
  });

  const arcs = $derived(tree ? sunburstArcs(tree, metric, depth) : []);
  const rootVal = $derived(tree ? (metric === 'size' ? tree.size : tree.alloc) : 0);

  function pickArc(a: RingArc, e: MouseEvent) {
    if (a.id < 0) return;
    storage.selected = e.ctrlKey ? new Set([...storage.selected, a.id]) : new Set([a.id]);
  }

  function zoomArc(a: RingArc) {
    if (a.is_dir && a.id >= 0) void storage.go(a.id);
  }

  function ctxArc(e: MouseEvent, a: RingArc) {
    if (a.id < 0) return;
    e.preventDefault();
    if (!storage.selected.has(a.id)) storage.selected = new Set([a.id]);
    oncontext?.(a.id, e.clientX, e.clientY);
  }
</script>

<div class="sb">
  <div class="bar">
    <label>Measure
      <select bind:value={metric}>
        <option value="size">File size</option>
        <option value="alloc">Space on disk</option>
      </select>
    </label>
    <label>Rings
      <input type="range" min="2" max="5" bind:value={depth} aria-label="Hierarchy rings" />
      <span class="tabular">{depth}</span>
    </label>
    <label>Colour by
      <select bind:value={colorMode}>
        <option value="folder">Top folder</option>
        <option value="category">Kind of file</option>
        <option value="age">Modified age</option>
        <option value="extension">Extension</option>
      </select>
    </label>
    <button class="plain" onclick={() => storage.up()} disabled={storage.detail?.node.parent == null}>Up one level</button>
    <span class="hint">Double-click a folder ring to zoom in. Click the centre to go up.</span>
  </div>

  <div class="stage">
    {#if arcs.length === 0}
      <p class="empty">{error || (storage.busy ? 'Looking…' : 'Nothing to draw here.')}</p>
    {:else}
      <div class="chartbox">
        <svg viewBox="-1.05 -1.05 2.1 2.1" role="img" aria-label="Sunburst ring chart of the current folder">
          <circle
            cx="0"
            cy="0"
            r="0.245"
            class="center"
            onclick={() => storage.up()}
            role="button"
            tabindex="0"
            aria-label="Go up one folder"
            onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && storage.up()}
          />
          {#each arcs as a (a.depth + '-' + a.id + '-' + a.path.slice(0, 24))}
            <path
              d={a.path}
              fill={resolveColor(a, colorMode)}
              class="arc"
              class:sel={a.id >= 0 && storage.selected.has(a.id)}
              onmouseenter={() => (hover = a)}
              onmouseleave={() => { if (hover === a) hover = null; }}
              onclick={(e) => pickArc(a, e)}
              ondblclick={() => zoomArc(a)}
              oncontextmenu={(e) => ctxArc(e, a)}
              role="button"
              tabindex="-1"
              aria-label={`${a.name}: ${formatUnitBytes(a.value, storage.unit)}`}
              onkeydown={() => {}}
            />
          {/each}
        </svg>
        <div class="hub">
          <strong title={hover ? hover.name : tree?.name}>{hover ? hover.name : tree?.name ?? ''}</strong>
          <span class="tabular">{formatUnitBytes(hover ? hover.value : rootVal, storage.unit)}</span>
          <span class="hint tabular">
            {#if hover}
              {hover.share * 100 < 0.1 ? '<0.1' : (hover.share * 100).toFixed(1)}% · level {hover.depth}
            {:else}
              {storage.detail?.node.parent != null ? 'Click centre to go up' : 'Root'}
            {/if}
          </span>
        </div>
      </div>

      <aside class="side">
        {#if colorMode === 'category'}
          <h4>Colours</h4>
          <ul class="legend">
            {#each Object.keys(CATEGORY_LABELS) as k (k)}
              <li><i style:background={CATEGORY_COLORS[k]}></i>{CATEGORY_LABELS[k]}</li>
            {/each}
          </ul>
        {:else if colorMode === 'age'}
          <h4>Modified age</h4>
          <ul class="legend">
            {#each AGE_PALETTE as b (b.label)}
              <li><i style:background={b.color}></i>{b.label}</li>
            {/each}
          </ul>
        {/if}
        <h4>Biggest in this ring</h4>
        <ul class="top">
          {#each arcs.filter((a) => a.depth === 1).slice(0, 10) as a (a.id + a.name)}
            <li>
              <button
                class="topbtn"
                onmouseenter={() => (hover = a)}
                onmouseleave={() => { if (hover === a) hover = null; }}
                onclick={() => (a.is_dir && a.id >= 0 ? storage.go(a.id) : a.id >= 0 && (storage.selected = new Set([a.id])))}
              >
                <i style:background={resolveColor(a, colorMode)}></i>
                <span class="nm" title={a.name}>{a.name}</span>
                <span class="tabular">{formatUnitBytes(a.value, storage.unit)}</span>
              </button>
            </li>
          {/each}
        </ul>
      </aside>
    {/if}
  </div>
</div>

<style>
  .sb { display: grid; grid-template-rows: auto 1fr; height: 100%; min-height: 0; gap: 6px; padding: 6px 8px; }
  .bar { display: flex; flex-wrap: wrap; gap: 8px 16px; align-items: center; }
  label { display: flex; gap: 6px; align-items: center; color: var(--text-2); font-size: 12.5px; }
  select { padding: 4px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  input[type='range'] { accent-color: var(--accent); width: 84px; }
  .hint { color: var(--text-3); font-size: 12px; }
  .stage { display: grid; grid-template-columns: minmax(240px, 1fr) 250px; gap: 16px; min-height: 0; align-items: center; }
  .chartbox { position: relative; height: 100%; min-height: 220px; display: grid; place-items: center; }
  svg { width: min(100%, 420px); height: min(100%, 420px); overflow: visible; }
  .center { fill: var(--surface-2); stroke: var(--border-strong); stroke-width: 0.01; cursor: pointer; }
  .center:hover { fill: var(--surface-3); }
  .arc { stroke: var(--surface-1); stroke-width: 0.008; cursor: pointer; transition: opacity var(--dur-1); }
  .arc:hover { opacity: 0.85; stroke: var(--text); stroke-width: 0.014; }
  .arc.sel { stroke: var(--text); stroke-width: 0.018; }
  .hub { position: absolute; width: 112px; text-align: center; pointer-events: none; display: grid; gap: 2px; font-size: 12px; }
  .hub strong { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .side { align-self: stretch; overflow: auto; border-left: 1px solid var(--border); padding-left: 12px; display: grid; align-content: start; gap: 8px; }
  h4 { margin: 0; font-size: 12.5px; color: var(--text-2); }
  .legend, .top { list-style: none; margin: 0; padding: 0; display: grid; gap: 3px; font-size: 12px; }
  .legend li { display: flex; gap: 6px; align-items: center; color: var(--text-2); }
  i { width: 10px; height: 10px; border-radius: 2px; flex: none; display: inline-block; }
  .topbtn { display: grid; grid-template-columns: 10px 1fr auto; gap: 6px; align-items: center; width: 100%; border: 0; background: none; color: inherit; padding: 3px 4px; border-radius: 6px; cursor: pointer; text-align: left; }
  .topbtn:hover { background: var(--surface-2); }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { grid-column: 1 / -1; text-align: center; color: var(--text-3); margin: 0; }
  .plain { padding: 4px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  .plain:disabled { opacity: 0.5; cursor: default; }
</style>
