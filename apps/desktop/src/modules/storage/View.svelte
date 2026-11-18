<script lang="ts">
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { stIpc, type StError } from '../../lib/ipc';
  import ContextMenu, { type MenuItem } from '../../lib/ui/ContextMenu.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import Charts from './Charts.svelte';
  import DriveCards from './DriveCards.svelte';
  import Duplicates from './Duplicates.svelte';
  import History from './History.svelte';
  import Search from './Search.svelte';
  import FileTable from './FileTable.svelte';
  import OpDialogs from './OpDialogs.svelte';
  import Rankings from './Rankings.svelte';
  import Sunburst from './Sunburst.svelte';
  import Tree from './Tree.svelte';
  import TreeGrid from './TreeGrid.svelte';
  import Treemap from './Treemap.svelte';
  import {
    completenessNote,
    formatElapsed,
    formatUnitBytes,
    shortcutAction,
    shortcutsActive,
    stateLabel,
    type DisplayUnit,
  } from './logic';
  import { storage } from './store.svelte';

  let dropHover = $state(false);

  $effect(() => {
    void storage.init();
    void storage.loadDrives();
  });

  // Allow dropping a folder or saved .json scan file onto the Disk Analyzer window.
  $effect(() => {
    let off: (() => void) | null = null;
    let gone = false;
    void getCurrentWebview()
      .onDragDropEvent((ev) => {
        if (ev.payload.type === 'enter' || ev.payload.type === 'over') {
          dropHover = true;
        } else if (ev.payload.type === 'leave') {
          dropHover = false;
        } else if (ev.payload.type === 'drop') {
          dropHover = false;
          const first = ev.payload.paths[0];
          if (!first) return;
          if (first.toLowerCase().endsWith('.json')) {
            void stIpc.loadScan(first).then(async (id) => {
              const prog = await stIpc.progress(id);
              if (!storage.sessions.some((s) => s.id === id)) storage.sessions = [...storage.sessions, prog];
              await storage.select(id);
            }).catch((e) => (storage.error = String(e)));
          } else {
            void storage.start(first, false);
          }
        }
      })
      .then((unlisten) => {
        if (gone) unlisten();
        else off = unlisten;
      })
      .catch(() => {});
    return () => {
      gone = true;
      off?.();
    };
  });

  const p = $derived(storage.progress);
  const note = $derived(p ? completenessNote(p, storage.wholeDrive) : null);
  let errors = $state<StError[] | null>(null);

  async function showErrors() {
    if (storage.sessionId == null) return;
    errors = errors ? null : await stIpc.errors(storage.sessionId);
  }

  const trail = $derived(storage.detail?.trail ?? []);
  const root = $derived(storage.detail ? storage.detail.trail[0] : null);
  const count = $derived(storage.selected.size);
  const opBusy = $derived(storage.opProgress != null && storage.dialog?.type !== 'confirm');

  function onKeys(e: KeyboardEvent) {
    const t = e.target as HTMLElement | null;
    const field = !!t && (t.tagName === 'INPUT' || t.tagName === 'TEXTAREA' || t.tagName === 'SELECT' || t.isContentEditable);
    const action = shortcutAction({ key: e.key, ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey, alt: e.altKey });
    if (!action) return;
    const ok = shortcutsActive({
      dialogOpen: storage.dialog != null,
      menuOpen: storage.menu != null,
      targetIsField: field,
      tab: storage.tab,
      hasSession: storage.sessionId != null,
      busy: storage.busy || storage.opProgress != null,
    });
    if (!ok && !(action === 'refresh' && storage.sessionId != null && !field && !storage.busy && storage.dialog == null)) return;
    if (action === 'selectAll' && t?.closest('[role=listbox]')) return;
    e.preventDefault();
    run(action);
  }

  function run(action: string) {
    switch (action) {
      case 'recycle': if (count) void storage.remove(false); break;
      case 'delete': if (count) void storage.remove(true); break;
      case 'copy': storage.copySelection(false); break;
      case 'cut': storage.copySelection(true); break;
      case 'paste': void storage.paste(); break;
      case 'rename': storage.startRename(); break;
      case 'refresh': void storage.refresh(); break;
      case 'properties': void storage.showProperties(); break;
      case 'selectAll': storage.selectAll(); break;
    }
  }

  const menuItems = $derived.by((): MenuItem[] => {
    const one = count === 1;
    const sel = storage.selectedRows;
    const folder = one && sel[0]?.is_dir && !sel[0].is_link;
    const items: MenuItem[] = [
      { id: 'open', label: folder ? 'Open folder' : 'Open', disabled: !one, hint: 'Enter' },
      { id: 'reveal', label: 'Show in Explorer', disabled: !one },
      { id: 'copypath', label: 'Copy path', disabled: count === 0 },
      { id: 'copy', label: 'Copy', hint: 'Ctrl+C', separatorBefore: true, disabled: count === 0 },
      { id: 'cut', label: 'Cut', hint: 'Ctrl+X', disabled: count === 0 },
      { id: 'paste', label: 'Paste here', hint: 'Ctrl+V', disabled: !storage.clipboard },
      { id: 'copyto', label: 'Copy to…', disabled: count === 0 },
      { id: 'moveto', label: 'Move to…', disabled: count === 0 },
      { id: 'rename', label: 'Rename', hint: 'F2', separatorBefore: true, disabled: !one },
      { id: 'batchrename', label: 'Rename in batch…', disabled: count < 2 },
      { id: 'zip', label: 'Add to ZIP file…', disabled: count === 0 },
      { id: 'compress', label: 'Compress on disk (NTFS)…', separatorBefore: true, disabled: count === 0 },
      { id: 'decompress', label: 'Uncompress (NTFS)…', disabled: count === 0 },
      { id: 'savepaths', label: 'Save list of paths…', disabled: count === 0 },
      { id: 'recycle', label: 'Move to Recycle Bin', hint: 'Delete', separatorBefore: true, disabled: count === 0 },
      { id: 'delete', label: 'Delete permanently…', hint: 'Shift+Delete', disabled: count === 0 },
      { id: 'rescan', label: 'Scan this folder again', separatorBefore: true, disabled: !folder },
      { id: 'properties', label: 'Properties', hint: 'Alt+Enter' },
    ];
    return items;
  });

  async function pick(id: string) {
    storage.menu = null;
    switch (id) {
      case 'open': {
        const n = storage.selectedRows[0];
        if (n?.is_dir && !n.is_link) await storage.go(n.id);
        else await storage.openSelected(false);
        break;
      }
      case 'reveal': await storage.openSelected(true); break;
      case 'copypath': await storage.copyPath(); break;
      case 'copyto': await storage.sendTo('copy'); break;
      case 'moveto': await storage.sendTo('move'); break;
      case 'batchrename': storage.startBatchRename(); break;
      case 'zip': storage.startZip(); break;
      case 'compress': await storage.compressSelection(true); break;
      case 'decompress': await storage.compressSelection(false); break;
      case 'savepaths': await storage.savePaths(); break;
      case 'rescan': {
        const n = storage.selectedRows[0];
        if (n && storage.sessionId != null) await stIpc.rescan(storage.sessionId, n.id).catch((e) => (storage.error = String(e)));
        break;
      }
      default: run(id);
    }
  }

  const UNITS: DisplayUnit[] = ['auto', 'GB', 'MB', 'KB', 'B'];
</script>

<svelte:window onkeydown={onKeys} />

<div class="page" class:dropping={dropHover}>
  <h1>Disk Analyzer</h1>

  {#if storage.sessionId == null}
    <DriveCards />
    <section class="card note">
      <p>NomNom reads folder names, sizes, owners and dates without changing anything while scanning. Drop a folder here or pick a drive above to begin. Deleting, moving, compressing and hardlink deduplication happen only after a preview and your confirmation.</p>
    </section>
  {:else if p}
    <div class="top">
      <div class="sessions" role="tablist" aria-label="Open scans">
        {#each storage.sessions as s (s.id)}
          <span class="chip" class:on={s.id === storage.sessionId}>
            <button role="tab" aria-selected={s.id === storage.sessionId} onclick={() => storage.select(s.id)}>
              {s.root}{#if s.elevated} <span class="adm">Admin</span>{/if}
            </button>
            <button class="x" aria-label={`Close the scan of ${s.root}`} onclick={() => storage.close(s.id)}>×</button>
          </span>
        {/each}
        <button class="plain" onclick={() => (storage.sessionId = null)}>New scan…</button>
      </div>
      <div class="actions">
        <label class="unitpick" title="Display unit">
          <select value={storage.unit} onchange={(e) => (storage.unit = e.currentTarget.value as DisplayUnit)} aria-label="Size unit">
            {#each UNITS as u (u)}<option value={u}>{u === 'auto' ? 'Unit: Auto' : u}</option>{/each}
          </select>
        </label>
        {#if p.state === 'running'}
          <button class="plain" onclick={() => storage.pause()}>Pause</button>
        {:else if p.state === 'paused'}
          <button class="plain" onclick={() => storage.resume()}>Continue</button>
        {/if}
        {#if storage.busy}
          <button class="plain" onclick={() => storage.cancel()}>Stop</button>
        {:else}
          <button class="plain" onclick={() => storage.refresh()} title="Scan this folder again (F5)">Refresh</button>
        {/if}
        <button class="plain" onclick={() => storage.saveScanFile()} disabled={storage.busy}>Save scan…</button>
        <button class="plain" onclick={() => storage.startExport()} disabled={storage.busy}>Save report…</button>
      </div>
    </div>

    {#if storage.busy}
      <div class="progress"><ProgressBar value={storage.wholeDrive && p.drive_used ? Math.min(0.99, p.bytes / p.drive_used) : null} label="Scan progress" /></div>
    {/if}

    <div class="tabs" role="tablist">
      <button role="tab" aria-selected={storage.tab === 'explorer'} class:on={storage.tab === 'explorer'} onclick={() => (storage.tab = 'explorer')}>Explorer</button>
      <button role="tab" aria-selected={storage.tab === 'rankings'} class:on={storage.tab === 'rankings'} onclick={() => (storage.tab = 'rankings')}>Biggest items</button>
      <button role="tab" aria-selected={storage.tab === 'charts'} class:on={storage.tab === 'charts'} onclick={() => (storage.tab = 'charts')}>Charts</button>
      <button role="tab" aria-selected={storage.tab === 'search'} class:on={storage.tab === 'search'} onclick={() => (storage.tab = 'search')}>Search</button>
      <button role="tab" aria-selected={storage.tab === 'duplicates'} class:on={storage.tab === 'duplicates'} onclick={() => (storage.tab = 'duplicates')}>Duplicates</button>
      <button role="tab" aria-selected={storage.tab === 'history'} class:on={storage.tab === 'history'} onclick={() => (storage.tab = 'history')}>History</button>
    </div>

    {#if storage.error}<p class="error" role="alert">{storage.error} <button class="link" onclick={() => (storage.error = '')}>Dismiss</button></p>{/if}

    <div class="body">
      {#if storage.tab === 'explorer'}
        <div class="explorer" class:fullwidth={storage.view === 'treegrid'}>
          {#if storage.view !== 'treegrid'}
            <aside class="tree" aria-label="Folders">
              {#if root}<Tree node={root} expandedRoot={true} />{/if}
            </aside>
          {/if}
          <div class="main">
            <div class="bar">
              <nav class="crumbs" aria-label="Location">
                {#each trail as t, i (t.id)}
                  {#if i > 0}<span class="sep" aria-hidden="true">›</span>{/if}
                  <button class="crumb" onclick={() => storage.go(t.id)} aria-current={i === trail.length - 1 ? 'location' : undefined}>{i === 0 ? p.root : t.name}</button>
                {/each}
              </nav>
              <div class="right">
                <div class="seg" role="radiogroup" aria-label="View">
                  <button role="radio" aria-checked={storage.view === 'treegrid'} class:on={storage.view === 'treegrid'} onclick={() => (storage.view = 'treegrid')}>Tree</button>
                  <button role="radio" aria-checked={storage.view === 'table'} class:on={storage.view === 'table'} onclick={() => (storage.view = 'table')}>Split list</button>
                  <button role="radio" aria-checked={storage.view === 'treemap'} class:on={storage.view === 'treemap'} onclick={() => (storage.view = 'treemap')}>Treemap</button>
                  <button role="radio" aria-checked={storage.view === 'sunburst'} class:on={storage.view === 'sunburst'} onclick={() => (storage.view = 'sunburst')}>Sunburst</button>
                </div>
                {#if storage.view === 'table' || storage.view === 'treegrid'}
                  <input class="filter" placeholder="Filter by name" aria-label="Filter this folder" value={storage.text} oninput={(e) => storage.setText(e.currentTarget.value)} />
                {/if}
              </div>
            </div>
            <div class="grid">
              {#if storage.view === 'treegrid'}
                <TreeGrid />
              {:else if storage.view === 'table'}
                <FileTable />
              {:else if storage.view === 'treemap'}
                <Treemap oncontext={(_, x, y) => (storage.menu = { x, y })} />
              {:else}
                <Sunburst oncontext={(_, x, y) => (storage.menu = { x, y })} />
              {/if}
            </div>
          </div>
        </div>
      {:else if storage.tab === 'rankings'}
        <Rankings />
      {:else if storage.tab === 'search'}
        <Search />
      {:else if storage.tab === 'duplicates'}
        <Duplicates />
      {:else if storage.tab === 'history'}
        <History />
      {:else}
        <Charts />
      {/if}
    </div>

    <div class="status" role="status" aria-live="polite">
      <span class="state">{stateLabel(p)}</span>
      <span class="tabular">{p.files.toLocaleString()} files</span>
      <span class="tabular">{p.dirs.toLocaleString()} folders</span>
      <span class="tabular">{formatUnitBytes(p.bytes, storage.unit)} ({formatUnitBytes(p.allocated, storage.unit)} on disk)</span>
      <span class="tabular">{formatElapsed(p.elapsed_ms)}</span>
      {#if count > 0}<span class="tabular">{count.toLocaleString()} selected</span>{/if}
      {#if p.errors > 0}<button class="link" onclick={showErrors}>{p.errors.toLocaleString()} {p.errors === 1 ? 'place' : 'places'} couldn't be read</button>{/if}
      {#if !p.elevated && !storage.busy && p.errors > 0}
        <button class="link" onclick={() => storage.start(p.root, true)}>Scan as Administrator</button>
      {/if}
      {#if storage.busy}<span class="partial">Results so far. Totals will change.</span>{/if}
      {#if opBusy && storage.opProgress}<span class="partial">Working: {storage.opProgress.done} of {storage.opProgress.total} <button class="link" onclick={() => storage.cancelOp()}>Stop</button></span>{/if}
      {#if storage.notice}<span class="notice-inline">{storage.notice}</span>{/if}
    </div>
    {#if note}<p class="notice">{note}</p>{/if}
    {#if errors}
      <div class="errlist" role="region" aria-label="Places that couldn't be read">
        {#each errors.slice(0, 200) as e (e.path)}<div><span class="mono">{e.path}</span> <span class="hint">{e.message}</span></div>{/each}
        {#if errors.length > 200}<div class="hint">and {errors.length - 200} more</div>{/if}
        {#if !p.elevated}
          <p class="hint">NomNom runs without administrator rights by default. Use "Scan as Administrator" if you want to read protected Windows folders too.</p>
        {/if}
      </div>
    {/if}
  {/if}
</div>

{#if storage.menu}
  <ContextMenu x={storage.menu.x} y={storage.menu.y} items={menuItems} onpick={pick} onclose={() => (storage.menu = null)} />
{/if}
<OpDialogs />

<style>
  .page.dropping { outline: 2px dashed var(--accent); outline-offset: 6px; border-radius: var(--radius-m); }
  h1 { margin-bottom: 14px; }
  .card { max-width: 780px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 12px 16px; }
  .note p { margin: 0; color: var(--text-2); }
  .top { display: flex; justify-content: space-between; gap: 12px; flex-wrap: wrap; margin-bottom: 8px; }
  .sessions, .actions, .right { display: flex; gap: 8px; flex-wrap: wrap; align-items: center; }
  .chip { display: inline-flex; border: 1px solid var(--border-strong); border-radius: var(--radius-s); overflow: hidden; background: var(--surface-2); }
  .chip.on { border-color: var(--accent); }
  .chip button { border: 0; background: none; color: inherit; padding: 6px 10px; cursor: pointer; display: inline-flex; gap: 6px; align-items: center; }
  .chip .x { padding: 6px 8px; color: var(--text-3); }
  .adm { font-size: 11px; padding: 0 5px; border-radius: 4px; background: var(--surface-3); color: var(--accent-text); }
  .unitpick select { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); cursor: pointer; }
  .progress { margin-bottom: 8px; }
  .tabs { display: flex; gap: 4px; border-bottom: 1px solid var(--border); margin-bottom: 8px; }
  .tabs button { border: 0; background: none; color: var(--text-2); padding: 7px 14px; cursor: pointer; border-bottom: 2px solid transparent; }
  .tabs button.on { color: var(--text); border-bottom-color: var(--accent); font-weight: 600; }
  .body { height: calc(100vh - 330px); min-height: 280px; }
  .explorer { display: grid; grid-template-columns: 250px 1fr; gap: 12px; height: 100%; min-height: 0; }
  .explorer.fullwidth { grid-template-columns: 1fr; }
  .tree { overflow: auto; border: 1px solid var(--border); border-radius: var(--radius-s); background: var(--surface-1); padding: 4px; }
  .main { display: grid; grid-template-rows: auto 1fr; min-width: 0; min-height: 0; gap: 6px; }
  .bar { display: flex; gap: 10px; justify-content: space-between; align-items: center; flex-wrap: wrap; }
  .crumbs { display: flex; flex-wrap: wrap; align-items: center; gap: 2px; min-width: 0; }
  .crumb { border: 0; background: none; color: var(--accent-text); cursor: pointer; padding: 2px 4px; border-radius: 4px; }
  .crumb[aria-current='location'] { color: var(--text); font-weight: 600; }
  .sep { color: var(--text-3); }
  .seg { display: inline-flex; border: 1px solid var(--border-strong); border-radius: var(--radius-s); overflow: hidden; }
  .seg button { border: 0; background: var(--surface-2); color: inherit; padding: 5px 12px; cursor: pointer; }
  .seg button.on { background: var(--accent); color: var(--on-accent); font-weight: 600; }
  .filter { padding: 5px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); min-width: 170px; }
  .grid { min-height: 0; border: 1px solid var(--border); border-radius: var(--radius-s); background: var(--surface-1); overflow: hidden; }
  .status { display: flex; gap: 16px; align-items: center; flex-wrap: wrap; padding: 8px 4px 0; color: var(--text-2); }
  .state { font-weight: 600; color: var(--text); }
  .partial { color: var(--info); }
  .notice-inline { color: var(--ok); }
  .notice { margin: 4px; color: var(--text-3); font-size: 12.5px; max-width: 920px; }
  .error { color: var(--danger); }
  .errlist { max-height: 160px; overflow: auto; border: 1px solid var(--border); border-radius: var(--radius-s); padding: 8px; margin-top: 6px; }
  .mono { font-family: Consolas, monospace; user-select: text; }
  .hint { color: var(--text-3); font-size: 12.5px; }
  button.plain, .actions button { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button.plain:hover { background: var(--surface-3); }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; cursor: pointer; padding: 0; }
</style>
