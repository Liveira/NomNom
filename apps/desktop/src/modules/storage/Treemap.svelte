<script lang="ts">
  import { stIpc, type StTree } from '../../lib/ipc';
  import { driveAccounting, formatUnitBytes } from './logic';
  import {
    AGE_PALETTE,
    CATEGORY_COLORS,
    CATEGORY_LABELS,
    hitTest,
    layoutTree,
    resolveColor,
    type Cell,
    type ColorMode,
    type Metric,
  } from './treemap';
  import { storage } from './store.svelte';

  let { oncontext }: { oncontext?: (id: number, x: number, y: number) => void } = $props();

  let canvas = $state<HTMLCanvasElement | null>(null);
  let wrap = $state<HTMLDivElement | null>(null);
  let width = $state(600);
  let height = $state(400);
  let tree = $state<StTree | null>(null);
  let cells = $state<Cell[]>([]);
  let metric = $state<Metric>('size');
  let colorMode = $state<ColorMode>('category');
  let cushioned = $state(true);
  let showDriveSpace = $state(false);
  let depth = $state(3);
  let hover = $state<Cell | null>(null);
  let tip = $state({ x: 0, y: 0 });
  let error = $state('');
  let seq = 0;

  async function load() {
    const id = storage.sessionId;
    if (id == null) return;
    const mine = ++seq;
    try {
      const t = await stIpc.treemap(id, storage.current, depth, 220);
      if (mine === seq) {
        tree = t;
        error = '';
      }
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    void [storage.sessionId, storage.current, storage.version, depth];
    void load();
  });

  $effect(() => {
    const acct = showDriveSpace && storage.current === 0 ? driveAccounting(storage.progress, storage.wholeDrive) : null;
    const extra = acct ? { freeBytes: acct.freeBytes, systemBytes: acct.systemBytes } : undefined;
    cells = tree ? layoutTree(tree, { x: 0, y: 0, w: width, h: height }, metric, 3, extra) : [];
  });

  const css = (name: string) => getComputedStyle(document.documentElement).getPropertyValue(name).trim() || '#888';

  function draw() {
    const c = canvas;
    if (!c) return;
    const dpr = window.devicePixelRatio || 1;
    c.width = Math.floor(width * dpr);
    c.height = Math.floor(height * dpr);
    const g = c.getContext('2d')!;
    g.scale(dpr, dpr);
    g.clearRect(0, 0, width, height);
    const bg = css('--surface-1');
    const text = css('--text');
    const nowUnix = Math.floor(Date.now() / 1000);
    g.font = '12px "Segoe UI Variable Text", "Segoe UI", sans-serif';
    g.textBaseline = 'top';
    for (const cell of cells) {
      const r = cell.rect;
      const base = resolveColor(cell, colorMode, nowUnix);
      g.globalAlpha = cell.is_dir ? 0.25 + Math.min(cell.depth, 3) * 0.05 : 0.92;
      g.fillStyle = base;
      g.fillRect(r.x, r.y, r.w, r.h);
      g.globalAlpha = 1;

      if (cushioned && !cell.is_dir && r.w > 8 && r.h > 8) {
        const grad = g.createLinearGradient(r.x, r.y, r.x + r.w, r.y + r.h);
        grad.addColorStop(0, 'rgba(255, 255, 255, 0.22)');
        grad.addColorStop(0.45, 'rgba(255, 255, 255, 0.02)');
        grad.addColorStop(1, 'rgba(0, 0, 0, 0.28)');
        g.fillStyle = grad;
        g.fillRect(r.x, r.y, r.w, r.h);
      }

      g.strokeStyle = bg;
      g.lineWidth = 1;
      g.strokeRect(r.x + 0.5, r.y + 0.5, r.w - 1, r.h - 1);
      if (storage.selected.has(cell.id) && cell.id >= 0) {
        g.strokeStyle = text;
        g.lineWidth = 2;
        g.strokeRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
      }
      if (r.w > 46 && r.h > 16 && cell.depth > 0) {
        g.fillStyle = cell.is_dir ? text : '#1a1320';
        g.save();
        g.beginPath();
        g.rect(r.x + 2, r.y, r.w - 4, r.h);
        g.clip();
        g.fillText(cell.name, r.x + 4, r.y + 2);
        if (!cell.is_dir && r.h > 32) g.fillText(formatUnitBytes(cell.value, storage.unit), r.x + 4, r.y + 16);
        g.restore();
      }
    }
    if (hover) {
      const r = hover.rect;
      g.strokeStyle = css('--accent');
      g.lineWidth = 2;
      g.strokeRect(r.x + 1, r.y + 1, r.w - 2, r.h - 2);
    }
  }

  $effect(() => {
    void [cells, hover, width, height, storage.selected, colorMode, cushioned, storage.unit];
    draw();
  });

  function point(e: MouseEvent) {
    const r = canvas!.getBoundingClientRect();
    return { x: e.clientX - r.left, y: e.clientY - r.top };
  }
  function over(e: MouseEvent) {
    const p = point(e);
    hover = hitTest(cells, p.x, p.y);
    tip = p;
  }
  function click(e: MouseEvent) {
    const p = point(e);
    const c = hitTest(cells, p.x, p.y);
    if (!c || c.id < 0) return;
    if (c.id === storage.current) return;
    storage.selected = e.ctrlKey ? new Set([...storage.selected, c.id]) : new Set([c.id]);
  }
  function dbl(e: MouseEvent) {
    const p = point(e);
    const c = hitTest(cells, p.x, p.y);
    if (c && c.is_dir && c.id >= 0) void storage.go(c.id);
  }
  function ctx(e: MouseEvent) {
    e.preventDefault();
    const p = point(e);
    const c = hitTest(cells, p.x, p.y);
    if (c && c.id >= 0 && c.id !== storage.current) {
      if (!storage.selected.has(c.id)) storage.selected = new Set([c.id]);
      oncontext?.(c.id, e.clientX, e.clientY);
    }
  }

  const legend = Object.keys(CATEGORY_LABELS);
  const rootVal = $derived(cells[0]?.value ?? (tree ? (metric === 'size' ? tree.size : tree.alloc) : 1));
  const share = $derived(hover ? (hover.value / Math.max(1, rootVal)) * 100 : 0);
</script>

<div class="tm">
  <div class="bar">
    <label>Measure
      <select bind:value={metric}>
        <option value="size">File size</option>
        <option value="alloc">Space on disk</option>
      </select>
    </label>
    <label>Colour by
      <select bind:value={colorMode}>
        <option value="category">Kind of file</option>
        <option value="age">Modified age</option>
        <option value="folder">Top folder</option>
        <option value="extension">Extension</option>
      </select>
    </label>
    <label>Levels
      <input type="range" min="1" max="5" bind:value={depth} aria-label="Levels of detail" />
      <span class="tabular">{depth}</span>
    </label>
    <label class="chk"><input type="checkbox" bind:checked={cushioned} /> Cushioned shading</label>
    {#if storage.wholeDrive && storage.current === 0}
      <label class="chk"><input type="checkbox" bind:checked={showDriveSpace} /> Free and system space</label>
    {/if}
    <button class="plain" onclick={() => storage.up()} disabled={storage.detail?.node.parent == null}>Up one level</button>
    {#if colorMode === 'category'}
      <ul class="legend" aria-label="Colours">
        {#each legend as k (k)}<li><i style:background={CATEGORY_COLORS[k]}></i>{CATEGORY_LABELS[k]}</li>{/each}
      </ul>
    {:else if colorMode === 'age'}
      <ul class="legend" aria-label="Modified age colours">
        {#each AGE_PALETTE as b (b.label)}<li><i style:background={b.color}></i>{b.label}</li>{/each}
      </ul>
    {/if}
  </div>
  <div class="canvaswrap" bind:this={wrap} bind:clientWidth={width} bind:clientHeight={height}>
    <canvas
      bind:this={canvas}
      style:width="{width}px"
      style:height="{height}px"
      onmousemove={over}
      onmouseleave={() => (hover = null)}
      onclick={click}
      ondblclick={dbl}
      oncontextmenu={ctx}
      aria-label="Treemap of the current folder. Use the table for keyboard access."
    ></canvas>
    {#if hover && hover.id !== undefined}
      <div class="tip" style:left="{Math.min(tip.x + 14, width - 240)}px" style:top="{Math.min(tip.y + 14, height - 76)}px">
        <strong>{hover.name}</strong>
        <div class="tabular">{formatUnitBytes(hover.value, storage.unit)}{tree && hover.id !== tree.id ? ` · ${share < 0.1 ? '<0.1' : share.toFixed(1)}%` : ''}</div>
        <div class="hint">
          {hover.folded ? 'Folded together to keep the picture readable' : hover.id === -3 ? 'Free drive space' : hover.id === -2 ? 'System or unopened space' : hover.is_dir ? 'Folder (double-click to open)' : (CATEGORY_LABELS[hover.category] ?? 'File')}
        </div>
      </div>
    {/if}
    {#if cells.length === 0}<p class="empty">{error || (storage.busy ? 'Looking…' : 'Nothing to draw here.')}</p>{/if}
  </div>
</div>

<style>
  .tm { display: grid; grid-template-rows: auto 1fr; height: 100%; min-height: 0; gap: 6px; padding: 6px 8px; }
  .bar { display: flex; flex-wrap: wrap; gap: 6px 14px; align-items: center; }
  label { display: flex; gap: 6px; align-items: center; color: var(--text-2); font-size: 12.5px; }
  .chk { cursor: pointer; }
  select { padding: 4px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  input[type='range'] { accent-color: var(--accent); width: 76px; }
  .hint { color: var(--text-3); font-size: 12px; }
  .legend { display: flex; flex-wrap: wrap; gap: 4px 10px; list-style: none; margin: 0; padding: 0; font-size: 12px; color: var(--text-2); }
  .legend li { display: flex; gap: 5px; align-items: center; }
  .legend i { width: 10px; height: 10px; border-radius: 2px; display: inline-block; }
  .canvaswrap { position: relative; min-height: 0; border: 1px solid var(--border); border-radius: var(--radius-s); background: var(--surface-1); overflow: hidden; }
  canvas { display: block; }
  .tip { position: absolute; pointer-events: none; background: var(--surface-2); border: 1px solid var(--border-strong); border-radius: var(--radius-s); padding: 6px 10px; max-width: 230px; box-shadow: 0 6px 20px rgba(0, 0, 0, 0.35); }
  .tip strong { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .empty { position: absolute; inset: 0; display: grid; place-items: center; color: var(--text-3); margin: 0; }
  .plain { padding: 4px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  .plain:disabled { opacity: 0.5; cursor: default; }
</style>
