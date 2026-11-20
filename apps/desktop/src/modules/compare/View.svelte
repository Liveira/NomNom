<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import { compareIpc, type CompareApplied, type CompareRow } from '../../lib/ipc';
  import Dialog from '../../lib/ui/Dialog.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import { STATUS_LABEL, counts, leaf, matchesFilter, replaces, suggest, type Action, type Direction, type Filter } from './logic';

  const SHOWN = 400;

  let left = $state('');
  let right = $state('');
  let exact = $state(false);
  let rows = $state<CompareRow[] | null>(null);
  let truncated = $state(false);
  let filter = $state<Filter>('changes');
  let dir = $state<Direction>('to_right');
  /** The rows that will be copied, by path. */
  let chosen = $state<Record<string, Action>>({});
  let busy = $state(false);
  let error = $state('');
  let confirming = $state(false);
  let result = $state<CompareApplied | null>(null);

  const tally = $derived(rows ? counts(rows) : null);
  const visible = $derived((rows ?? []).filter((r) => matchesFilter(r, filter)));
  const plan = $derived((rows ?? []).filter((r) => chosen[r.rel]));
  const replaceCount = $derived(plan.filter((r) => replaces(r, chosen[r.rel])).length);

  async function pick(set: (p: string) => void) {
    const p = await open({ directory: true, multiple: false });
    if (typeof p === 'string') set(p);
  }

  /** The default choice: what a sync in the chosen direction would copy. */
  function preselect() {
    chosen = Object.fromEntries((rows ?? []).flatMap((r) => {
      const a = suggest(r, dir);
      return a ? [[r.rel, a] as const] : [];
    }));
  }

  async function compare() {
    busy = true;
    error = '';
    result = null;
    try {
      const s = await compareIpc.scan(left, right, exact);
      rows = s.rows;
      truncated = s.truncated;
      filter = 'changes';
      preselect();
    } catch (e) {
      error = String(e);
      rows = null;
    } finally {
      busy = false;
    }
  }

  function toggle(r: CompareRow, action: Action) {
    const next = { ...chosen };
    if (next[r.rel] === action) delete next[r.rel];
    else next[r.rel] = action;
    chosen = next;
  }

  async function apply() {
    confirming = false;
    busy = true;
    error = '';
    try {
      result = await compareIpc.apply(left, right, plan.map((r) => ({ rel: r.rel, action: chosen[r.rel] })));
      await compare();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }

  const when = (s: number) => new Date(s * 1000).toLocaleString(undefined, { day: 'numeric', month: 'short', year: 'numeric', hour: '2-digit', minute: '2-digit' });
  const FILTERS: { id: Filter; label: string }[] = [
    { id: 'changes', label: 'All changes' },
    { id: 'only_left', label: 'Only on the left' },
    { id: 'only_right', label: 'Only on the right' },
    { id: 'different', label: 'Different' },
    { id: 'same', label: 'The same' },
  ];
</script>

<header class="head">
  <h1><Icon name="arrow" size={24} /> Compare folders</h1>
  <p class="sub">See what is different between two folders, then copy what you choose. Nothing is ever deleted. When a copy would replace a file, the old one is kept in a folder called .nomnom-replaced.</p>
</header>

<section class="sides">
  <label>Left folder
    <span class="row"><input bind:value={left} spellcheck="false" translate="no" /><button class="plain" onclick={() => void pick((p) => (left = p))}>Choose…</button></span>
  </label>
  <label>Right folder
    <span class="row"><input bind:value={right} spellcheck="false" translate="no" /><button class="plain" onclick={() => void pick((p) => (right = p))}>Choose…</button></span>
  </label>
</section>
<div class="row opts">
  <span class="inline"><Switch checked={exact} label="Read files to be sure" onchange={(v) => (exact = v)} /> <span>Read the files to be sure (slower)</span></span>
  <button class="primary" disabled={busy || !left.trim() || !right.trim()} onclick={() => void compare()}>{busy ? 'Working…' : 'Compare'}</button>
</div>
{#if error}<p class="bad" role="alert">{error}</p>{/if}

{#if result}
  <p class="ok" role="status">
    {result.copied === 1 ? '1 file copied' : `${result.copied} files copied`}{result.replaced > 0 ? `, ${result.replaced} replaced (the old versions are in .nomnom-replaced)` : ''}.
    {#if result.failed.length > 0}<span class="bad">{result.failed.length} could not be copied: {result.failed.slice(0, 3).map(([p, m]) => `${leaf(p)} (${m})`).join('; ')}</span>{/if}
  </p>
{/if}

{#if rows && tally}
  {#if truncated}<p class="hint">One of the folders has more files than are compared at once, so only the first {100000} are here.</p>{/if}
  <div class="bar">
    <div class="seg" role="radiogroup" aria-label="Show">
      {#each FILTERS as f (f.id)}
        <button role="radio" aria-checked={filter === f.id} class:on={filter === f.id} onclick={() => (filter = f.id)}>{f.label} <span class="n tabular">{tally[f.id]}</span></button>
      {/each}
    </div>
    <label class="inline">Copy
      <select bind:value={dir} onchange={preselect} aria-label="Direction">
        <option value="to_right">to the right folder</option>
        <option value="to_left">to the left folder</option>
        <option value="both">both ways, the newer wins</option>
      </select>
    </label>
  </div>

  {#if visible.length === 0}
    <p class="hint">{rows.length === 0 ? 'Both folders are empty.' : 'Nothing here.'}</p>
  {:else}
    <ul class="list">
      {#each visible.slice(0, SHOWN) as r (r.rel)}
        <li>
          <div class="body">
            <strong translate="no" title={r.rel}>{leaf(r.rel)}</strong>
            <span class="path" translate="no">{r.rel}</span>
            <span class="st">{STATUS_LABEL[r.status]}</span>
          </div>
          <div class="side tabular">{#if r.left}<span>{formatBytes(r.left[0])}</span><span class="hint">{when(r.left[1])}</span>{:else}<span class="hint">—</span>{/if}</div>
          <div class="side tabular">{#if r.right}<span>{formatBytes(r.right[0])}</span><span class="hint">{when(r.right[1])}</span>{:else}<span class="hint">—</span>{/if}</div>
          <div class="acts">
            <button class="chip" class:on={chosen[r.rel] === 'copy_right'} disabled={!r.left} aria-pressed={chosen[r.rel] === 'copy_right'} onclick={() => toggle(r, 'copy_right')}>Copy →</button>
            <button class="chip" class:on={chosen[r.rel] === 'copy_left'} disabled={!r.right} aria-pressed={chosen[r.rel] === 'copy_left'} onclick={() => toggle(r, 'copy_left')}>← Copy</button>
          </div>
        </li>
      {/each}
    </ul>
    {#if visible.length > SHOWN}<p class="hint">Showing the first {SHOWN} of {visible.length}. The choices above apply to all of them.</p>{/if}
  {/if}

  <div class="foot">
    <span class="hint">{plan.length === 0 ? 'Nothing chosen to copy.' : `${plan.length} to copy${replaceCount > 0 ? `, ${replaceCount} would replace a file` : ''}.`}</span>
    <button class="primary" disabled={busy || plan.length === 0} onclick={() => (confirming = true)}>Copy the chosen files…</button>
  </div>
{/if}

{#if confirming}
  <Dialog title="Copy these files?" onclose={() => (confirming = false)}>
    <p>{plan.length === 1 ? '1 file will be copied' : `${plan.length} files will be copied`}.</p>
    {#if replaceCount > 0}<p>{replaceCount === 1 ? '1 of them replaces a file that is already there' : `${replaceCount} of them replace files that are already there`}. The old versions are not deleted: they go to a folder called .nomnom-replaced next to them.</p>{/if}
    <p class="hint">Nothing is deleted from either folder.</p>
    {#snippet actions()}
      <button class="plain" onclick={() => (confirming = false)}>Cancel</button>
      <button class="primary" onclick={() => void apply()}>Copy</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .ok { color: var(--ok); font-size: 13px; }
  .sides { display: grid; grid-template-columns: repeat(auto-fit, minmax(320px, 1fr)); gap: 12px; max-width: 900px; }
  label { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  label.inline, .inline { display: flex; align-items: center; gap: 8px; font-weight: 600; font-size: 13px; }
  input, select { font: inherit; padding: 8px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); min-width: 0; box-sizing: border-box; }
  .row { display: flex; gap: 8px; align-items: center; }
  .row input { flex: 1; }
  .opts { margin: 12px 0; justify-content: space-between; max-width: 900px; flex-wrap: wrap; }
  .bar { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; margin: 16px 0 10px; max-width: 1000px; }
  .seg { display: inline-flex; flex-wrap: wrap; padding: 3px; border-radius: 18px; background: var(--surface-2); border: 1px solid var(--border); }
  .seg button { border: 0; background: none; color: var(--text-2); padding: 5px 12px; border-radius: 99px; cursor: pointer; font: inherit; font-size: 12.5px; font-weight: 600; }
  .seg button.on { background: var(--surface-3); color: var(--text); }
  .n { margin-left: 4px; color: var(--text-3); }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; max-width: 1000px; }
  .list li { display: grid; grid-template-columns: minmax(0, 1.6fr) minmax(0, 1fr) minmax(0, 1fr) auto; gap: 12px; align-items: center; padding: 8px 12px; border-radius: var(--radius-s); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); }
  .body { display: grid; gap: 1px; min-width: 0; }
  .body strong, .path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .path { color: var(--text-3); font: 11.5px var(--font-mono); }
  .st { font-size: 12px; color: var(--accent-text); font-weight: 700; }
  .side { display: grid; font-size: 12.5px; }
  .acts { display: flex; gap: 6px; }
  .chip { border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text-2); border-radius: 99px; padding: 4px 11px; font: inherit; font-size: 12px; font-weight: 600; cursor: pointer; }
  .chip.on { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  .chip:disabled { opacity: 0.35; cursor: default; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 12px; margin-top: 14px; max-width: 1000px; }
  button.plain, button.primary { display: inline-flex; align-items: center; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:focus-visible, input:focus-visible, select:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
