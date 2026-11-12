<script lang="ts">
  import VirtualList from '../../lib/ui/VirtualList.svelte';
  import { formatBytes } from '../../lib/format';
  import { stIpc, type StFilter, type StNode } from '../../lib/ipc';
  import { formatDate, OVERLAP_NOTE, parseSize } from './logic';
  import { storage } from './store.svelte';

  let what = $state<'files' | 'folders'>('files');
  let mode = $state<'inclusive' | 'exclusive'>('inclusive');
  let count = $state(100);
  let scope = $state<'all' | 'here'>('all');
  let minText = $state('');
  let minUnit = $state('MB');
  let maxText = $state('');
  let maxUnit = $state('MB');
  let extText = $state('');
  let category = $state('');
  let age = $state(''); // '', 'recent:30', 'old:365'
  let items = $state<StNode[]>([]);
  let paths = $state<Record<number, string>>({});
  let error = $state('');
  let loading = $state(false);
  let seq = 0;

  async function run() {
    const id = storage.sessionId;
    if (id == null) return;
    const min = parseSize(minText, minUnit);
    const max = parseSize(maxText, maxUnit);
    if (min === 'bad' || max === 'bad') {
      error = 'Sizes must be plain numbers, like 500 or 1.5.';
      return;
    }
    error = '';
    const filter: StFilter = {};
    if (min != null) filter.min_size = min;
    if (max != null) filter.max_size = max;
    const exts = extText.split(/[\s,;]+/).map((e) => e.replace(/^\./, '').toLowerCase()).filter(Boolean);
    if (exts.length) filter.extensions = exts;
    if (category) filter.category = category;
    if (age) {
      const [kind, days] = age.split(':');
      const cutoff = Math.floor(Date.now() / 1000) - Number(days) * 86400;
      if (kind === 'recent') filter.modified_after = cutoff;
      else filter.modified_before = cutoff;
    }
    const node = scope === 'here' ? storage.current : 0;
    const mine = ++seq;
    loading = true;
    try {
      const r =
        what === 'files'
          ? await stIpc.topFiles(id, node, count, filter)
          : await stIpc.topFolders(id, node, count, mode, min != null ? min : undefined);
      if (mine === seq) {
        items = r;
        const list = await stIpc.paths(id, r.map((n) => n.id));
        if (mine === seq) paths = Object.fromEntries(r.map((n, i) => [n.id, list[i]]));
      }
    } catch (e) {
      error = String(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  // The charts can send us here already filtered by a kind or an extension.
  $effect(() => {
    if (storage.rankingCategory || storage.rankingExtension) {
      what = 'files';
      category = storage.rankingCategory;
      extText = storage.rankingExtension;
      storage.rankingCategory = '';
      storage.rankingExtension = '';
    }
  });

  // Run again whenever a control changes, and when the scan's data changed (partial or finished).
  $effect(() => {
    void [what, mode, count, scope, minText, minUnit, maxText, maxUnit, extText, category, age, storage.sessionId, storage.version, storage.current];
    const t = setTimeout(() => void run(), 150);
    return () => clearTimeout(t);
  });

  async function show(n: StNode) {
    storage.tab = 'explorer';
    await storage.go(n.is_dir ? n.id : (n.parent ?? 0));
    if (!n.is_dir) storage.selected = new Set([n.id]);
  }

  const total = $derived(items.reduce((s, i) => s + i.size, 0));
</script>

<div class="wrap">
  <div class="controls" role="group" aria-label="Ranking options">
    <div class="seg" role="radiogroup" aria-label="What to rank">
      <button role="radio" aria-checked={what === 'files'} class:on={what === 'files'} onclick={() => (what = 'files')}>Biggest files</button>
      <button role="radio" aria-checked={what === 'folders'} class:on={what === 'folders'} onclick={() => (what = 'folders')}>Biggest folders</button>
    </div>
    <label>Show
      <select bind:value={count}>
        {#each [10, 50, 100, 500, 5000] as n (n)}<option value={n}>{n === 5000 ? 'up to 5000' : `top ${n}`}</option>{/each}
      </select>
    </label>
    <label>Where
      <select bind:value={scope}>
        <option value="all">Whole scan</option>
        <option value="here">Current folder only</option>
      </select>
    </label>
    {#if what === 'folders'}
      <label>Size of a folder
        <select bind:value={mode}>
          <option value="inclusive">Everything inside</option>
          <option value="exclusive">Files directly inside</option>
        </select>
      </label>
    {/if}
    <label>Bigger than
      <span class="inline"><input class="num" bind:value={minText} inputmode="decimal" aria-label="Minimum size" />
        <select bind:value={minUnit} aria-label="Unit"><option>KB</option><option>MB</option><option>GB</option></select></span>
    </label>
    {#if what === 'files'}
      <label>Smaller than
        <span class="inline"><input class="num" bind:value={maxText} inputmode="decimal" aria-label="Maximum size" />
          <select bind:value={maxUnit} aria-label="Unit"><option>KB</option><option>MB</option><option>GB</option></select></span>
      </label>
      <label>Types
        <input class="ext" bind:value={extText} placeholder="mp4, zip" aria-label="File extensions" />
      </label>
      <label>Kind
        <select bind:value={category}>
          <option value="">Any</option>
          {#each ['video', 'image', 'audio', 'document', 'executable', 'archive', 'temp', 'other'] as c (c)}<option value={c}>{c[0].toUpperCase() + c.slice(1)}</option>{/each}
        </select>
      </label>
      <label>Modified
        <select bind:value={age}>
          <option value="">Any time</option>
          <option value="recent:7">In the last 7 days</option>
          <option value="recent:30">In the last 30 days</option>
          <option value="recent:90">In the last 90 days</option>
          <option value="old:365">Over 1 year ago</option>
          <option value="old:1095">Over 3 years ago</option>
        </select>
      </label>
    {/if}
  </div>

  {#if what === 'folders' && mode === 'inclusive'}<p class="note" role="note">{OVERLAP_NOTE}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="head" role="row">
    <span class="rank">#</span><span class="n">Name and location</span><span class="s right">Size</span><span class="a right">On disk</span><span class="m">Modified</span><span class="go"></span>
  </div>
  <div class="list">
    {#if items.length === 0}
      <p class="empty">{loading || storage.busy ? 'Looking…' : 'Nothing matches these filters.'}</p>
    {:else}
      <VirtualList {items} rowHeight={34} kind="list" label={what === 'files' ? 'Biggest files' : 'Biggest folders'}>
        {#snippet row(n: StNode, i: number)}
          <div class="tr">
            <span class="rank tabular">{i + 1}</span>
            <span class="n"><span class="nm" title={n.name}>{n.name}</span><span class="where" title={paths[n.id]}>{paths[n.id] ?? ''}{n.is_dir ? ` · ${n.files.toLocaleString()} files, ${n.dirs.toLocaleString()} folders` : ''}</span></span>
            <span class="s right tabular">{formatBytes(n.size)}</span>
            <span class="a right tabular">{what === 'files' ? formatBytes(n.alloc) : ''}</span>
            <span class="m">{formatDate(n.mtime)}</span>
            <span class="go"><button class="link" onclick={() => show(n)}>Show in folder</button></span>
          </div>
        {/snippet}
      </VirtualList>
    {/if}
  </div>
  <p class="foot tabular">
    {items.length.toLocaleString()} {what === 'files' ? 'files' : 'folders'} · {formatBytes(total)}{what === 'folders' && mode === 'inclusive' ? ' (overlapping, not a real total)' : ''}
  </p>
</div>

<style>
  .wrap { display: grid; grid-template-rows: auto auto auto 1fr auto; height: 100%; min-height: 0; gap: 8px; }
  .controls { display: flex; flex-wrap: wrap; gap: 10px 16px; align-items: end; }
  label { display: grid; gap: 3px; color: var(--text-2); font-size: 12.5px; }
  .inline { display: inline-flex; gap: 4px; }
  .num { width: 70px; } .ext { width: 110px; }
  input, select { padding: 5px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .seg { display: inline-flex; border: 1px solid var(--border-strong); border-radius: var(--radius-s); overflow: hidden; }
  .seg button { border: 0; background: var(--surface-2); color: inherit; padding: 6px 12px; cursor: pointer; }
  .seg button.on { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .note { margin: 0; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); color: var(--text-2); }
  .error { color: var(--danger); margin: 0; }
  .head, .tr { display: grid; grid-template-columns: 44px 1fr 100px 100px 110px 120px; align-items: center; gap: 8px; padding: 0 8px; }
  .head { color: var(--text-2); font-weight: 600; border-bottom: 1px solid var(--border); padding-bottom: 4px; }
  .tr { height: 100%; border-radius: 4px; }
  .tr:hover { background: var(--surface-2); }
  .right { text-align: right; }
  .n { min-width: 0; display: grid; }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
  .where { color: var(--text-3); font-size: 11.5px; }
  .list { min-height: 0; }
  .empty { color: var(--text-3); margin: 8px; }
  .foot { margin: 0; color: var(--text-3); }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; cursor: pointer; padding: 0; }
</style>
