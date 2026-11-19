<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { renameIpc, type RenameItem, type RenameOptions } from '../../lib/ipc';
  import { PATTERNS, baseName, countStatus, defaultOptions } from './rename';

  let paths = $state<string[]>([]);
  let options = $state<RenameOptions>(defaultOptions());
  let items = $state<RenameItem[]>([]);
  let busy = $state(false);
  let error = $state('');
  let done = $state('');
  let canUndo = $state(0);
  let timer: ReturnType<typeof setTimeout> | undefined;

  const counts = $derived(countStatus(items));
  const canApply = $derived(!busy && counts.ok > 0 && counts.conflict === 0 && counts.invalid === 0);

  async function refreshUndo() {
    canUndo = await renameIpc.canUndo().catch(() => 0);
  }
  void refreshUndo();

  async function choose() {
    error = '';
    done = '';
    const picked = await open({ multiple: true });
    if (!picked) return;
    const list = Array.isArray(picked) ? picked : [picked];
    try {
      paths = await renameIpc.inspect(list);
    } catch (e) {
      error = String(e);
      paths = [];
    }
  }

  // The preview follows every change, a moment after the last key.
  $effect(() => {
    const snapshot = JSON.stringify([paths, options]);
    void snapshot;
    clearTimeout(timer);
    if (paths.length === 0) {
      items = [];
      return;
    }
    timer = setTimeout(async () => {
      try {
        items = await renameIpc.preview(paths, $state.snapshot(options));
        error = '';
      } catch (e) {
        error = String(e);
        items = [];
      }
    }, 180);
    return () => clearTimeout(timer);
  });

  async function apply() {
    busy = true;
    error = '';
    try {
      const r = await renameIpc.apply(paths, $state.snapshot(options));
      done = `Renamed ${r.renamed} ${r.renamed === 1 ? 'file' : 'files'}.${r.failed.length ? ` ${r.failed.length} could not be renamed.` : ''}`;
      if (r.failed.length) error = r.failed.map(([p, e]) => `${baseName(p)}: ${e}`).join('\n');
      paths = [];
      items = [];
      await refreshUndo();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  async function undo() {
    busy = true;
    error = '';
    try {
      const r = await renameIpc.undo();
      done = `Put back ${r.renamed} ${r.renamed === 1 ? 'name' : 'names'}.`;
      await refreshUndo();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  function insert(token: string) {
    options.template += token;
  }
</script>

<section class="card rename" aria-labelledby="rn-h">
  <div class="head">
    <h2 id="rn-h"><Icon name="pencil" size={18} /> Rename many files</h2>
    <div class="head-actions">
      {#if canUndo > 0}<button class="plain" onclick={undo} disabled={busy}>Undo the last rename ({canUndo})</button>{/if}
      <button class="plain" onclick={choose} disabled={busy}>Choose files…</button>
    </div>
  </div>
  <p class="hint">Make a pattern, check the new names, then apply. Each file keeps its own extension, and nothing is ever overwritten.</p>

  {#if paths.length > 0}
    <div class="grid">
      <label class="wide">
        <span>New name</span>
        <input type="text" bind:value={options.template} spellcheck="false" placeholder="{'{date}'}_{'{name}'}" aria-label="New name pattern" />
      </label>
      <div class="chips wide" aria-label="Pieces you can add">
        {#each PATTERNS as p (p.token)}
          <button class="chip" type="button" title={p.hint} onclick={() => insert(p.token)}>{p.token}</button>
        {/each}
      </div>
      <label><span>Find in the old name</span><input type="text" bind:value={options.find} spellcheck="false" /></label>
      <label><span>Replace it with</span><input type="text" bind:value={options.replace} spellcheck="false" /></label>
      <label>
        <span>Letters</span>
        <select bind:value={options.case}>
          <option value="keep">As they are</option>
          <option value="lower">all lowercase</option>
          <option value="upper">ALL UPPERCASE</option>
          <option value="title">Each Word Capitalised</option>
        </select>
      </label>
      <label>
        <span>Spaces</span>
        <select bind:value={options.spaces}>
          <option value="keep">Keep them</option>
          <option value="underscore">Use _</option>
          <option value="dash">Use -</option>
          <option value="dot">Use .</option>
          <option value="remove">Remove them</option>
        </select>
      </label>
      <label>
        <span>Number the files in order of</span>
        <select bind:value={options.order}>
          <option value="given">The list</option>
          <option value="name">Name</option>
          <option value="modified">Date changed</option>
        </select>
      </label>
      <label class="small"><span>First number</span><input type="number" min="0" bind:value={options.start} /></label>
      <label class="small"><span>Step</span><input type="number" min="1" bind:value={options.step} /></label>
      <label class="check"><input type="checkbox" bind:checked={options.lower_ext} /> <span>Lowercase extensions (.JPG to .jpg)</span></label>
    </div>

    <div class="summary" role="status">
      {counts.ok} will change{#if counts.same > 0}, {counts.same} stay the same{/if}{#if counts.conflict > 0}, {counts.conflict} clash{/if}{#if counts.invalid > 0}, {counts.invalid} cannot be used{/if}
    </div>

    <div class="table" role="table" aria-label="Preview of the new names">
      {#each items.slice(0, 200) as it (it.from)}
        <div class="row {it.status}" role="row" title={it.note}>
          <span class="from" translate="no">{it.from_name}</span>
          <Icon name="arrow" size={14} />
          <span class="to" translate="no">{it.to_name}</span>
          {#if it.note}<span class="note">{it.note}</span>{/if}
        </div>
      {/each}
      {#if items.length > 200}<div class="more">…and {items.length - 200} more with the same pattern.</div>{/if}
    </div>

    <div class="actions">
      <button class="plain" onclick={() => { paths = []; items = []; }} disabled={busy}>Clear</button>
      <button class="primary" onclick={apply} disabled={!canApply}>Rename {counts.ok} {counts.ok === 1 ? 'file' : 'files'}</button>
    </div>
  {/if}

  {#if done}<p class="done" role="status">{done}</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .rename { margin-top: 16px; }
  .head { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; }
  .head h2 { display: flex; align-items: center; gap: 8px; margin: 0; }
  .head-actions { display: flex; gap: 8px; }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 6px 0 12px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 12px; }
  .wide { grid-column: 1 / -1; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-3); font-weight: 600; }
  label.check { display: flex; align-items: center; gap: 8px; align-self: end; }
  label.small { max-width: 120px; }
  input[type='text'], input[type='number'], select { background: var(--surface-2); color: var(--text); border: 1px solid var(--border-strong); border-radius: var(--radius-xs); padding: 8px 10px; font: inherit; }
  input[type='text'] { width: 100%; box-sizing: border-box; font-family: var(--font-mono); }
  .chips { display: flex; flex-wrap: wrap; gap: 6px; }
  .chip { font-family: var(--font-mono); font-size: 12px; padding: 4px 9px; border-radius: 99px; border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--accent-text); cursor: pointer; }
  .chip:hover { background: var(--surface-3); }
  .summary { margin: 14px 0 8px; color: var(--text-2); font-size: 13px; }
  .table { display: grid; gap: 4px; max-height: 320px; overflow: auto; padding-right: 4px; }
  .row { display: flex; align-items: center; gap: 10px; padding: 6px 10px; border-radius: var(--radius-xs); background: var(--surface-2); font-size: 13px; min-width: 0; }
  .row span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .from { color: var(--text-3); flex: 1 1 0; }
  .to { flex: 1 1 0; font-weight: 600; }
  .row.same .to { color: var(--text-3); font-weight: 400; }
  .row.conflict, .row.invalid { border: 1px solid var(--danger); }
  .row.conflict .to, .row.invalid .to { color: var(--danger); }
  .note { color: var(--danger); font-size: 11.5px; flex: none; }
  .more { color: var(--text-3); font-size: 12.5px; padding: 4px 10px; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 12px; }
  button.plain, button.primary { padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.plain:hover:not(:disabled) { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.5; cursor: default; }
  .done { color: var(--ok); margin: 10px 0 0; }
  .error { color: var(--danger); margin: 10px 0 0; white-space: pre-line; }
</style>
