<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import VirtualList from '../../lib/ui/VirtualList.svelte';
  import { formatBytes } from '../../lib/format';
  import type { StNode } from '../../lib/ipc';
  import { COLUMNS, defaultColumnState, formatDate, moveColumn, parseColumnState, visibleColumns, type ColumnState } from './logic';
  import { storage } from './store.svelte';

  const KEY = 'nomnom.storage.columns';
  let cols = $state<ColumnState>(defaultColumnState());
  try {
    cols = parseColumnState(localStorage.getItem(KEY));
  } catch {
    /* storage can be unavailable */
  }
  function saveCols() {
    try {
      localStorage.setItem(KEY, JSON.stringify(cols));
    } catch {
      /* storage can be unavailable */
    }
  }

  const shown = $derived(visibleColumns(cols));
  const template = $derived(shown.map((c) => `${c.w}px`).join(' '));
  const totalWidth = $derived(shown.reduce((n, c) => n + c.w, 0));
  let menuOpen = $state(false);
  let cursor = $state(-1);
  let list = $state<ReturnType<typeof VirtualList<StNode>> | null>(null);
  let dragKey = $state<string | null>(null);

  function startResize(e: PointerEvent, key: string, startW: number) {
    e.preventDefault();
    e.stopPropagation();
    const x0 = e.clientX;
    const move = (ev: PointerEvent) => {
      cols.widths = { ...cols.widths, [key]: Math.max(50, Math.round(startW + ev.clientX - x0)) };
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
      saveCols();
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  }

  function toggleColumn(key: string) {
    cols.hidden = cols.hidden.includes(key) ? cols.hidden.filter((k) => k !== key) : [...cols.hidden, key];
    saveCols();
  }

  function drop(before: string) {
    if (dragKey) {
      cols.order = moveColumn(cols.order, dragKey, before);
      saveCols();
    }
    dragKey = null;
  }

  function open(n: StNode) {
    if (n.is_dir && !n.is_link) void storage.go(n.id);
  }

  function onKey(e: KeyboardEvent) {
    const n = storage.rows.length;
    if (e.target instanceof HTMLInputElement) return;
    const mods = { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey };
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      if (n === 0) return;
      const next = e.key === 'Home' ? 0 : e.key === 'End' ? n - 1 : Math.min(n - 1, Math.max(0, (cursor < 0 ? (e.key === 'ArrowDown' ? -1 : n) : cursor) + (e.key === 'ArrowDown' ? 1 : -1)));
      cursor = next;
      storage.clickRow(next, { ctrl: false, shift: mods.shift });
      list?.reveal(next);
    } else if (e.key === 'Enter' && cursor >= 0) {
      e.preventDefault();
      open(storage.rows[cursor]);
    } else if (e.key === 'Backspace') {
      e.preventDefault();
      void storage.up();
    } else if ((e.key === 'a' || e.key === 'A') && mods.ctrl) {
      e.preventDefault();
      storage.selectAll();
    }
  }

  const cell = (c: string, n: StNode): string => {
    switch (c) {
      case 'size': return formatBytes(n.size);
      case 'alloc': return n.is_dir || n.alloc > 0 ? formatBytes(n.alloc) : '0 B';
      case 'files': return n.is_dir ? n.files.toLocaleString() : '';
      case 'dirs': return n.is_dir ? n.dirs.toLocaleString() : '';
      case 'modified': return formatDate(n.mtime);
      default: return '';
    }
  };
</script>

<div class="table" role="presentation">
  <div class="header" style:width="{totalWidth}px" style:grid-template-columns={template} role="row">
    {#each shown as c (c.key)}
      <div
        class="th"
        class:right={c.align === 'right'}
        role="columnheader"
        tabindex="-1"
        aria-sort={storage.sort === c.sort ? (storage.desc ? 'descending' : 'ascending') : 'none'}
        draggable="true"
        ondragstart={() => (dragKey = c.key)}
        ondragover={(e) => e.preventDefault()}
        ondrop={() => drop(c.key)}
      >
        <button class="sortbtn" disabled={!c.sort} onclick={() => c.sort && storage.setSort(c.sort)}>
          {c.label}{storage.sort === c.sort ? (storage.desc ? ' ▼' : ' ▲') : ''}
        </button>
        <span class="grip" role="separator" aria-label={`Resize ${c.label}`} onpointerdown={(e) => startResize(e, c.key, c.w)}></span>
      </div>
    {/each}
  </div>
  <div class="body" style:min-width="{totalWidth}px">
    {#if storage.rows.length === 0}
      <p class="empty">{storage.busy ? 'Looking…' : storage.text ? 'Nothing matches.' : 'This folder is empty.'}</p>
    {:else}
      <VirtualList bind:this={list} items={storage.rows} label="Folder contents" onkeydown={onKey}>
        {#snippet row(n: StNode, i: number)}
          <div
            class="tr"
            class:sel={storage.selected.has(n.id)}
            class:cur={i === cursor}
            style:grid-template-columns={template}
            style:width="{totalWidth}px"
            role="option"
            aria-selected={storage.selected.has(n.id)}
            tabindex="-1"
            onclick={(e) => {
              cursor = i;
              storage.clickRow(i, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
            }}
            ondblclick={() => open(n)}
            onkeydown={() => {}}
          >
            {#each shown as c (c.key)}
              {#if c.key === 'name'}
                <div class="td name" title={n.name}>
                  <Icon name={n.is_dir ? 'organizer' : 'convert'} size={14} />
                  <span class="text">{n.name}</span>
                  {#if n.is_link}<span class="tag">link</span>{/if}
                  {#if n.flags & 16}<span class="tag warn" title="Some of this folder couldn't be read">partial</span>{/if}
                  {#if n.flags & 4}<span class="tag" title="Another name for data already counted">same data</span>{/if}
                </div>
              {:else if c.key === 'percent'}
                <div class="td pct">
                  <div class="bar"><div style:width="{Math.min(100, n.percent)}%"></div></div>
                  <span class="tabular">{n.percent < 0.1 ? '<0.1' : n.percent.toFixed(1)}%</span>
                </div>
              {:else}
                <div class="td tabular" class:right={c.align === 'right'}>{cell(c.key, n)}</div>
              {/if}
            {/each}
          </div>
        {/snippet}
      </VirtualList>
    {/if}
  </div>
  <div class="foot">
    <button class="link" onclick={() => (menuOpen = !menuOpen)} aria-expanded={menuOpen}>Columns</button>
    {#if storage.truncated}<span class="hint">Showing the first {storage.rows.length.toLocaleString()} of {storage.total.toLocaleString()}. Use the filter to narrow the list.</span>{/if}
    {#if menuOpen}
      <div class="menu" role="group" aria-label="Choose columns">
        {#each COLUMNS as c (c.key)}
          <label><input type="checkbox" checked={c.key === 'name' || !cols.hidden.includes(c.key)} disabled={c.key === 'name'} onchange={() => toggleColumn(c.key)} /> {c.label}</label>
        {/each}
        <button class="link" onclick={() => { cols = defaultColumnState(); saveCols(); }}>Reset columns</button>
      </div>
    {/if}
  </div>
</div>

<style>
  .table { display: grid; grid-template-rows: auto 1fr auto; height: 100%; min-height: 0; overflow-x: auto; position: relative; }
  .header { display: grid; position: sticky; top: 0; background: var(--surface-1); border-bottom: 1px solid var(--border); }
  .th { position: relative; display: flex; align-items: center; padding: 0 4px; }
  .th.right { justify-content: flex-end; }
  .sortbtn { border: 0; background: none; color: var(--text-2); font-weight: 600; padding: 6px 4px; cursor: pointer; white-space: nowrap; }
  .sortbtn:disabled { cursor: default; }
  .grip { position: absolute; right: 0; top: 4px; bottom: 4px; width: 6px; cursor: col-resize; border-right: 1px solid var(--border-strong); }
  .body { min-height: 0; }
  .tr { display: grid; align-items: center; height: 100%; cursor: default; border-radius: 4px; }
  .tr:hover { background: var(--surface-2); }
  .tr.sel { background: var(--surface-3); }
  .tr.cur { outline: 1px dotted var(--text-3); outline-offset: -1px; }
  .td { padding: 0 8px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .td.right { text-align: right; }
  .name { display: flex; gap: 6px; align-items: center; user-select: text; }
  .text { overflow: hidden; text-overflow: ellipsis; }
  .tag { font-size: 11px; color: var(--info); border: 1px solid var(--border-strong); border-radius: 8px; padding: 0 6px; flex: none; }
  .tag.warn { color: var(--danger); }
  .pct { display: flex; gap: 8px; align-items: center; }
  .bar { flex: 1; height: 6px; border-radius: 3px; background: var(--surface-3); overflow: hidden; }
  .bar div { height: 100%; background: var(--accent); }
  .pct span { width: 44px; text-align: right; color: var(--text-2); font-size: 12px; }
  .empty { color: var(--text-3); padding: 16px; margin: 0; }
  .foot { display: flex; gap: 14px; align-items: center; padding: 6px 4px; border-top: 1px solid var(--border); position: relative; }
  .hint { color: var(--text-3); font-size: 12.5px; }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; cursor: pointer; padding: 0; }
  .menu { position: absolute; bottom: 100%; left: 4px; background: var(--surface-2); border: 1px solid var(--border-strong); border-radius: var(--radius-s); padding: 8px 12px; display: grid; gap: 4px; z-index: 5; }
  .menu label { display: flex; gap: 8px; align-items: center; }
</style>
