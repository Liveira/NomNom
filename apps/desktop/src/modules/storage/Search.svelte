<script lang="ts">
  import VirtualList from '../../lib/ui/VirtualList.svelte';
  import { formatBytes } from '../../lib/format';
  import { stIpc, stTools, type StFound, type StNode } from '../../lib/ipc';
  import { formatDate, nextSelection } from './logic';
  import { buildQuery, newRow, parseSaved, ROW_LABELS, type Row, type RowType, type Saved } from './queryBuilder';
  import { storage } from './store.svelte';

  const KEY = 'nomnom.storage.searches';
  const KINDS = ['video', 'image', 'audio', 'document', 'executable', 'archive', 'temp', 'other'];
  const ATTRS = ['hidden', 'system', 'readonly', 'compressed', 'encrypted', 'offline', 'sparse'];

  let rows = $state<Row[]>([newRow('name')]);
  let join = $state<'and' | 'or'>('and');
  let scope = $state<'all' | 'here'>('all');
  let saved = $state<Saved[]>([]);
  let saveName = $state('');
  let found = $state<StFound | null>(null);
  let paths = $state<Record<number, string>>({});
  let selected = $state(new Set<number>());
  let anchor = $state<number | null>(null);
  /** The row the keyboard is on. */
  let cursor = $state(-1);
  let list = $state<ReturnType<typeof VirtualList<StNode>> | null>(null);
  let error = $state('');
  let loading = $state(false);
  let ranOnce = $state(false);
  let seq = 0;

  function readSaved() {
    try {
      saved = parseSaved(localStorage.getItem(KEY));
    } catch {
      saved = [];
    }
  }
  function writeSaved() {
    try {
      localStorage.setItem(KEY, JSON.stringify(saved));
    } catch {
      error = "The search couldn't be saved on this computer.";
    }
  }
  $effect(readSaved);

  function add(type: RowType = 'name') {
    rows = [...rows, newRow(type)];
  }
  function drop(id: number) {
    rows = rows.filter((r) => r.id !== id);
  }
  function setType(r: Row, t: RowType) {
    rows = rows.map((x) => (x.id === r.id ? { ...newRow(t), id: r.id, not: r.not } : x));
  }

  async function run() {
    const id = storage.sessionId;
    if (id == null) return;
    const q = buildQuery(rows, join);
    if ('error' in q) {
      error = q.error;
      return;
    }
    error = '';
    const mine = ++seq;
    loading = true;
    try {
      const r = await stTools.search(id, scope === 'here' ? storage.current : 0, q.query, 5000);
      if (mine !== seq) return;
      found = r;
      ranOnce = true;
      selected = new Set();
      cursor = -1;
      anchor = null;
      const list = await stIpc.paths(id, r.items.map((n) => n.id));
      if (mine === seq) paths = Object.fromEntries(r.items.map((n, i) => [n.id, list[i]]));
    } catch (e) {
      if (mine === seq) error = String(e);
    } finally {
      if (mine === seq) loading = false;
    }
  }

  function save() {
    const name = saveName.trim();
    if (!name) return;
    const copy: Saved = { name, join, rows: rows.map((r) => ({ ...r })) };
    saved = [...saved.filter((s) => s.name !== name), copy];
    saveName = '';
    writeSaved();
  }
  function load(s: Saved) {
    rows = s.rows.map((r) => ({ ...r }));
    join = s.join;
    error = '';
  }
  function forget(name: string) {
    saved = saved.filter((s) => s.name !== name);
    writeSaved();
  }

  function pick(i: number, mods: { ctrl: boolean; shift: boolean }) {
    if (!found) return;
    cursor = i;
    const r = nextSelection(selected, anchor, found.items.map((x) => x.id), i, mods);
    selected = r.selected;
    anchor = r.anchor;
  }
  function click(i: number, e: MouseEvent) {
    pick(i, { ctrl: e.ctrlKey || e.metaKey, shift: e.shiftKey });
  }

  /** Arrow keys, Home, End, Ctrl+A and Enter on the list itself. Keys pressed on a button inside a row stay with the button. */
  function onKey(e: KeyboardEvent) {
    if (!found || e.target !== e.currentTarget) return;
    const n = found.items.length;
    if (n === 0) return;
    if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === 'Home' || e.key === 'End') {
      e.preventDefault();
      const next = e.key === 'Home' ? 0 : e.key === 'End' ? n - 1 : Math.min(n - 1, Math.max(0, (cursor < 0 ? (e.key === 'ArrowDown' ? -1 : n) : cursor) + (e.key === 'ArrowDown' ? 1 : -1)));
      pick(next, { ctrl: false, shift: e.shiftKey });
      list?.reveal(next);
    } else if ((e.key === 'a' || e.key === 'A') && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      selected = new Set(found.items.map((x) => x.id));
    } else if (e.key === 'Enter' && cursor >= 0) {
      e.preventDefault();
      void show(found.items[cursor]);
    }
  }

  const chosen = $derived([...selected]);

  async function show(n: StNode) {
    storage.tab = 'explorer';
    await storage.go(n.is_dir ? n.id : (n.parent ?? 0));
    if (!n.is_dir) storage.selected = new Set([n.id]);
  }

  async function afterAction() {
    // The plan finished or was cancelled; search again so removed items disappear from the list.
    if (ranOnce && storage.dialog == null) await run();
  }

  // When something changed on disk through NomNom, refresh the results.
  let lastVersion = storage.version;
  $effect(() => {
    const v = storage.version;
    if (v !== lastVersion && !storage.busy) {
      lastVersion = v;
      void afterAction();
    }
  });

  async function copyPaths() {
    const text = chosen.map((id) => paths[id]).filter(Boolean).join('\r\n');
    await navigator.clipboard.writeText(text);
    storage.say(chosen.length === 1 ? 'Path copied.' : `${chosen.length} paths copied.`);
  }
</script>

<div class="wrap">
  <form class="builder" onsubmit={(e) => { e.preventDefault(); void run(); }} aria-label="Search conditions">
    <div class="line">
      <span>Find items that match</span>
      <select bind:value={join} aria-label="How conditions combine">
        <option value="and">all of these</option>
        <option value="or">any of these</option>
      </select>
      <label class="plainlabel">in
        <select bind:value={scope} aria-label="Where to search">
          <option value="all">the whole scan</option>
          <option value="here">the current folder</option>
        </select>
      </label>
    </div>

    {#each rows as r (r.id)}
      <div class="cond">
        <label class="not"><input type="checkbox" bind:checked={r.not} /> not</label>
        <select value={r.type} onchange={(e) => setType(r, e.currentTarget.value as RowType)} aria-label="Condition">
          {#each Object.entries(ROW_LABELS) as [k, label] (k)}<option value={k}>{label}</option>{/each}
        </select>
        {#if r.type === 'name' || r.type === 'path'}
          <select bind:value={r.mode} aria-label="How to compare">
            <option value="contains">contains</option>
            <option value="wildcard">matches (* and ?)</option>
            <option value="regex">regular expression</option>
          </select>
          <input class="grow" bind:value={r.text} aria-label="Text" placeholder={r.mode === 'wildcard' ? '*.log' : r.mode === 'regex' ? '^backup-\\d+' : 'report'} />
        {:else if r.type === 'ext'}
          <input class="grow" bind:value={r.text} aria-label="File types" placeholder="mp4, mkv" />
        {:else if r.type === 'size'}
          <span class="pair">from <input class="num" bind:value={r.min} inputmode="decimal" aria-label="Smallest size" /> to <input class="num" bind:value={r.max} inputmode="decimal" aria-label="Biggest size" />
            <select bind:value={r.unit} aria-label="Unit"><option>KB</option><option>MB</option><option>GB</option></select></span>
        {:else if r.type === 'age'}
          <select bind:value={r.field} aria-label="Which date"><option value="modified">modified</option><option value="created">created</option><option value="accessed">last opened</option></select>
          <select bind:value={r.dir} aria-label="Older or newer"><option value="older">more than</option><option value="newer">less than</option></select>
          <input class="num" bind:value={r.days} inputmode="numeric" aria-label="Days" /> days ago
        {:else if r.type === 'date'}
          <select bind:value={r.field} aria-label="Which date"><option value="modified">modified</option><option value="created">created</option><option value="accessed">last opened</option></select>
          <span class="pair">from <input class="date" bind:value={r.after} placeholder="2024-01-31" aria-label="From date" /> to <input class="date" bind:value={r.before} placeholder="2024-12-31" aria-label="To date" /></span>
        {:else if r.type === 'attr'}
          <select bind:value={r.attr} aria-label="Attribute">{#each ATTRS as a (a)}<option value={a}>{a}</option>{/each}</select>
        {:else if r.type === 'kind'}
          <select bind:value={r.category} aria-label="Kind">{#each KINDS as k (k)}<option value={k}>{k[0].toUpperCase() + k.slice(1)}</option>{/each}</select>
        {:else}
          <select bind:value={r.what} aria-label="File or folder"><option value="file">files</option><option value="folder">folders</option></select>
        {/if}
        <button type="button" class="x" onclick={() => drop(r.id)} aria-label="Remove this condition" disabled={rows.length === 1}>×</button>
      </div>
    {/each}

    <div class="line">
      <button type="button" class="plain" onclick={() => add()}>Add a condition</button>
      <button type="submit" class="primary" disabled={loading || storage.sessionId == null}>{loading ? 'Searching…' : 'Search'}</button>
      <span class="spacer"></span>
      <input bind:value={saveName} placeholder="Name this search" aria-label="Name for this search" onkeydown={(e) => e.key === 'Enter' && (e.preventDefault(), save())} />
      <button type="button" class="plain" onclick={save} disabled={!saveName.trim()}>Save</button>
    </div>
    {#if saved.length}
      <div class="line saved" role="list" aria-label="Saved searches">
        {#each saved as s (s.name)}
          <span class="chip" role="listitem"><button type="button" onclick={() => load(s)}>{s.name}</button><button type="button" class="x" aria-label={`Delete the saved search ${s.name}`} onclick={() => forget(s.name)}>×</button></span>
        {/each}
      </div>
    {/if}
  </form>

  {#if error}<p class="error" role="alert">{error}</p>{/if}

  <div class="head" role="row"><span class="n">Name and location</span><span class="s right">Size</span><span class="m">Modified</span><span class="go"></span></div>
  <div class="list">
    {#if !found}
      <p class="empty">Build a search above, then press Search.</p>
    {:else if found.items.length === 0}
      <p class="empty">{loading ? 'Searching…' : 'Nothing matches.'}</p>
    {:else}
      <VirtualList bind:this={list} items={found.items} rowHeight={34} label="Search results" onkeydown={onKey}>
        {#snippet row(n: StNode, i: number)}
          <div class="tr" class:sel={selected.has(n.id)} class:cur={i === cursor} role="option" aria-selected={selected.has(n.id)} tabindex="-1" onclick={(e) => click(i, e)} ondblclick={() => show(n)} onkeydown={() => {}}>
            <span class="n"><span class="nm" title={n.name}>{n.name}{n.is_dir ? ' (folder)' : ''}</span><span class="where" title={paths[n.id]}>{paths[n.id] ?? ''}</span></span>
            <span class="s right tabular">{formatBytes(n.size)}</span>
            <span class="m">{formatDate(n.mtime)}</span>
            <span class="go"><button class="link" onclick={(e) => { e.stopPropagation(); void show(n); }}>Show in folder</button></span>
          </div>
        {/snippet}
      </VirtualList>
    {/if}
  </div>

  <div class="foot">
    <span class="tabular hint">
      {#if found}{found.total.toLocaleString()} {found.total === 1 ? 'match' : 'matches'}{found.truncated ? `, showing the first ${found.items.length.toLocaleString()}` : ''}{#if chosen.length} · {chosen.length.toLocaleString()} selected{/if}{/if}
    </span>
    <span class="acts">
      <button class="plain" disabled={!chosen.length} onclick={copyPaths}>Copy paths</button>
      <button class="plain" disabled={!chosen.length} onclick={() => storage.savePaths(chosen)}>Save list of paths…</button>
      <button class="plain" disabled={!chosen.length} onclick={() => storage.startZip(chosen)}>Add to ZIP…</button>
      <button class="plain" disabled={!chosen.length} onclick={() => storage.startBatchRename(chosen)}>Rename in batch…</button>
      <button class="plain" disabled={!chosen.length} onclick={() => storage.remove(false, chosen)}>Move to Recycle Bin</button>
      <button class="plain danger" disabled={!chosen.length} onclick={() => storage.remove(true, chosen)}>Delete permanently…</button>
    </span>
  </div>
</div>

<style>
  .wrap { display: grid; grid-template-rows: auto auto auto minmax(150px, 1fr) auto; height: 100%; min-height: 0; gap: 8px; overflow: auto; }
  .builder { display: grid; gap: 6px; }
  .line { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; color: var(--text-2); }
  .plainlabel { display: inline-flex; gap: 6px; align-items: center; }
  .spacer { flex: 1; }
  .cond { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; padding: 4px 6px; border: 1px solid var(--border); border-radius: var(--radius-s); background: var(--surface-1); }
  .not { display: inline-flex; gap: 4px; align-items: center; color: var(--text-2); }
  .pair { display: inline-flex; gap: 6px; align-items: center; color: var(--text-2); }
  .grow { flex: 1; min-width: 160px; }
  .num { width: 70px; } .date { width: 110px; }
  input:not([type='checkbox']), select { padding: 5px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  button { padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.danger { border-color: var(--danger); color: var(--danger); }
  button.x { padding: 4px 9px; color: var(--text-3); }
  .chip { display: inline-flex; border: 1px solid var(--border-strong); border-radius: var(--radius-s); overflow: hidden; background: var(--surface-2); }
  .chip button { border: 0; border-radius: 0; background: none; padding: 4px 10px; }
  .error { color: var(--danger); margin: 0; }
  .head, .tr { display: grid; grid-template-columns: 1fr 100px 110px 120px; align-items: center; gap: 8px; padding: 0 8px; }
  .head { color: var(--text-2); font-weight: 600; border-bottom: 1px solid var(--border); padding-bottom: 4px; }
  .tr { height: 100%; border-radius: 4px; cursor: default; }
  .tr:hover { background: var(--surface-2); }
  .tr.sel { background: var(--accent-soft, var(--surface-3)); outline: 1px solid var(--accent); outline-offset: -1px; }
  .tr.cur { outline: 1px dotted var(--text-3); outline-offset: -1px; }
  .tr.cur.sel { outline: 2px solid var(--accent); }
  .right { text-align: right; }
  .n { min-width: 0; display: grid; }
  .nm { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
  .where { color: var(--text-3); font-size: 11.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .list { min-height: 0; }
  .empty { color: var(--text-3); margin: 8px; }
  .foot { display: flex; justify-content: space-between; gap: 12px; flex-wrap: wrap; align-items: center; }
  .acts { display: flex; gap: 6px; flex-wrap: wrap; }
  .hint { color: var(--text-3); }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; padding: 0; }
</style>
