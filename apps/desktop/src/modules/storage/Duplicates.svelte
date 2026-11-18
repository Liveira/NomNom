<script lang="ts">
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { stTools, type StDupGroup, type StDupProgress } from '../../lib/ipc';
  import { formatUnitBytes, parseSize } from './logic';
  import {
    applyRuleToGroups,
    initialChoice,
    setKeep,
    summarize,
    toggleRemove,
    toRequest,
    type Choice,
    type KeepRule,
  } from './dupes';
  import { storage } from './store.svelte';

  let minText = $state('1');
  let minUnit = $state('MB');
  let scope = $state<'all' | 'here'>('all');
  let filterText = $state('');
  let rule = $state<KeepRule>('oldest');
  let folderPrefix = $state('');
  let progress = $state<StDupProgress | null>(null);
  let groups = $state<StDupGroup[]>([]);
  let choices = $state(new Map<number, Choice>());
  let running = $state(false);
  let error = $state('');
  let startedFor = $state<number | null>(null);
  let timer: ReturnType<typeof setInterval> | null = null;

  const STAGES: Record<StDupProgress['stage'], string> = {
    listing: 'Listing files of the same size…',
    comparing: 'Comparing the start and end of each file…',
    hashing: 'Reading the files to be sure…',
    done: 'Finished.',
    cancelled: 'Stopped.',
  };

  async function start() {
    const id = storage.sessionId;
    if (id == null) return;
    const min = parseSize(minText, minUnit);
    if (min === 'bad' || min == null) {
      error = 'Give the smallest file size as a plain number, like 1 or 0.5.';
      return;
    }
    error = '';
    groups = [];
    choices = new Map();
    try {
      await stTools.dupesStart(id, scope === 'here' ? storage.current : 0, min);
      startedFor = id;
      running = true;
      void poll();
      timer = setInterval(() => void poll(), 500);
    } catch (e) {
      error = String(e);
    }
  }

  async function poll() {
    const r = await stTools.dupesProgress().catch(() => null);
    if (!r) return;
    progress = r[1];
    if (r[1].stage === 'done' || r[1].stage === 'cancelled') {
      if (timer) clearInterval(timer);
      timer = null;
      running = false;
      groups = await stTools.dupesGroups(0, 500);
      choices = new Map(groups.map((g, i) => [i, initialChoice(g)]));
    }
  }

  async function stop() {
    await stTools.dupesCancel();
  }

  $effect(() => () => {
    if (timer) clearInterval(timer);
  });

  $effect(() => {
    if (startedFor != null && storage.sessionId !== startedFor && !running) {
      groups = [];
      choices = new Map();
      progress = null;
      startedFor = null;
    }
  });

  const sum = $derived(summarize(groups, choices));
  const recoverable = $derived(groups.reduce((s, g) => s + g.recoverable, 0));
  const filteredGroups = $derived.by(() => {
    const q = filterText.trim().toLowerCase();
    return groups
      .map((g, index) => ({ g, index }))
      .filter(({ g }) => !q || g.copies.some((c) => c.path.toLowerCase().includes(q)));
  });

  function applyRule() {
    choices = applyRuleToGroups(groups, choices, rule, folderPrefix);
  }

  function keep(i: number, node: number) {
    const c = choices.get(i);
    if (!c) return;
    choices = new Map(choices).set(i, setKeep(c, node));
  }
  function toggle(i: number, node: number) {
    const c = choices.get(i);
    if (!c) return;
    choices = new Map(choices).set(i, toggleRemove(c, node));
  }

  async function plan(permanent: boolean, hardlink = false) {
    const id = storage.sessionId;
    if (id == null) return;
    const req = toRequest(groups, choices);
    if (req.length === 0) return;
    try {
      const p = await stTools.dupesPlan(id, permanent, req, hardlink);
      storage.show(p);
    } catch (e) {
      error = String(e);
    }
  }

  let lastVersion = storage.version;
  $effect(() => {
    const v = storage.version;
    if (v !== lastVersion) {
      lastVersion = v;
      if (!running && groups.length > 0 && storage.dialog == null) void start();
    }
  });

  const when = (t: number | null) => (t == null ? 'unknown date' : new Date(t * 1000).toLocaleDateString());
</script>

<div class="wrap">
  <div class="controls">
    <label>Files bigger than
      <span class="inline"><input class="num" bind:value={minText} inputmode="decimal" aria-label="Smallest size" />
        <select bind:value={minUnit} aria-label="Unit"><option>KB</option><option>MB</option><option>GB</option></select></span>
    </label>
    <label>Where
      <select bind:value={scope}><option value="all">Whole scan</option><option value="here">Current folder only</option></select>
    </label>
    {#if running}
      <button class="plain" onclick={stop}>Stop</button>
    {:else}
      <button class="primary" onclick={start} disabled={storage.sessionId == null || storage.busy}>{groups.length || progress ? 'Search again' : 'Find duplicates'}</button>
    {/if}

    {#if groups.length > 0}
      <label>Auto-mark rule
        <select bind:value={rule}>
          <option value="oldest">Keep oldest copy</option>
          <option value="newest">Keep newest copy</option>
          <option value="shortest_path">Keep shortest path</option>
          <option value="longest_path">Keep longest path</option>
          <option value="in_folder">Keep copy inside folder…</option>
          <option value="clear">Clear all marks</option>
        </select>
      </label>
      {#if rule === 'in_folder'}
        <label>Preferred folder
          <input class="folderbox" bind:value={folderPrefix} placeholder="C:\Users\Me\Pictures" aria-label="Preferred folder prefix" />
        </label>
      {/if}
      <button class="plain" onclick={applyRule}>Apply rule</button>
      <label>Filter paths
        <input class="filterbox" bind:value={filterText} placeholder="Filter results" aria-label="Filter duplicate paths" />
      </label>
    {/if}
  </div>
  <p class="note" role="note">Two files are duplicates only when every byte is the same. The copy marked "Keep" is never deleted. On NTFS drives, you can also replace marked copies with hardlinks so every file path keeps working while sharing one copy of the data on disk.</p>
  {#if storage.busy}<p class="note" role="note">The scan is still running. Wait for it to finish for a complete answer.</p>{/if}
  {#if error}<p class="error" role="alert">{error}</p>{/if}

  {#if progress}
    <div class="prog" role="status" aria-live="polite">
      {#if running}
        <ProgressBar value={progress.stage === 'hashing' && progress.bytes_total ? progress.bytes_done / progress.bytes_total : null} label="Duplicate search progress" />
      {/if}
      <span>{STAGES[progress.stage]}</span>
      {#if progress.stage === 'hashing'}<span class="tabular">{progress.files_done.toLocaleString()} of {progress.files_total.toLocaleString()} files, {formatUnitBytes(progress.bytes_done, storage.unit)} of {formatUnitBytes(progress.bytes_total, storage.unit)}</span>{/if}
      <span class="tabular">{progress.groups.toLocaleString()} {progress.groups === 1 ? 'group' : 'groups'} found, {formatUnitBytes(progress.recoverable, storage.unit)} could be freed</span>
      {#if progress.unreadable > 0}<span class="warn">{progress.unreadable.toLocaleString()} files couldn't be read and were left out.</span>{/if}
    </div>
  {/if}

  <div class="list">
    {#if groups.length === 0}
      <p class="empty">{running ? '' : progress ? 'No duplicates found with these settings.' : 'Press Find duplicates to start.'}</p>
    {:else if filteredGroups.length === 0}
      <p class="empty">No duplicate groups match that filter.</p>
    {:else}
      <div role="list" aria-label="Duplicate groups">
        {#each filteredGroups as { g, index } (g.sha256 + g.copies[0].node)}
          {@const c = choices.get(index)}
          <div class="group" role="listitem">
            <div class="gh"><strong class="tabular">{formatUnitBytes(g.size, storage.unit)}</strong> each, {g.copies.length} copies, {formatUnitBytes(g.recoverable, storage.unit)} could be freed</div>
            {#each g.copies as x (x.node)}
              <div class="copy">
                <label class="keep"><input type="radio" name={`keep-${index}`} checked={c?.keep === x.node} onchange={() => keep(index, x.node)} /> Keep</label>
                <label class="rm"><input type="checkbox" checked={c?.remove.has(x.node) ?? false} disabled={c?.keep === x.node} onchange={() => toggle(index, x.node)} /> Mark</label>
                <span class="p" title={x.path}>{x.path}</span>
                <span class="d hint">{when(x.mtime)}{x.names > 1 ? ` · ${x.names} names for the same data` : ''}</span>
              </div>
            {/each}
          </div>
        {/each}
      </div>
      {#if groups.length >= 500}<p class="hint note">Showing the 500 groups that would free the most space. Remove or link some and search again to see more.</p>{/if}
    {/if}
  </div>

  <div class="foot">
    <span class="tabular hint">
      {#if groups.length}{groups.length.toLocaleString()} {groups.length === 1 ? 'group' : 'groups'}, {formatUnitBytes(recoverable, storage.unit)} could be freed. Marked: {sum.files.toLocaleString()} {sum.files === 1 ? 'file' : 'files'}, {formatUnitBytes(sum.bytes, storage.unit)} freed.{/if}
    </span>
    <span class="acts">
      <button class="plain" disabled={sum.files === 0} onclick={() => plan(false, true)} title="Replace marked copies with NTFS hardlinks so all paths remain accessible">Replace with hardlinks…</button>
      <button class="plain" disabled={sum.files === 0} onclick={() => plan(false, false)}>Move marked to Recycle Bin…</button>
      <button class="plain danger" disabled={sum.files === 0} onclick={() => plan(true, false)}>Delete marked permanently…</button>
    </span>
  </div>
</div>

<style>
  .wrap { display: grid; grid-template-rows: auto auto auto auto 1fr auto; height: 100%; min-height: 0; gap: 8px; }
  .controls { display: flex; flex-wrap: wrap; gap: 10px 14px; align-items: end; }
  label { display: grid; gap: 3px; color: var(--text-2); font-size: 12.5px; }
  .inline { display: inline-flex; gap: 4px; }
  .num { width: 70px; }
  .folderbox { width: 190px; }
  .filterbox { width: 140px; }
  input:not([type='checkbox']):not([type='radio']), select { padding: 5px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  button { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.danger { border-color: var(--danger); color: var(--danger); }
  .note { margin: 0; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); color: var(--text-2); }
  .error { color: var(--danger); margin: 0; }
  .prog { display: grid; gap: 4px; color: var(--text-2); }
  .warn { color: var(--danger); }
  .list { min-height: 0; overflow: auto; }
  .empty { color: var(--text-3); margin: 8px; }
  .group { padding: 6px 8px; border-bottom: 1px solid var(--border); }
  .gh { margin-bottom: 4px; color: var(--text-2); }
  .copy { display: grid; grid-template-columns: 70px 80px 1fr auto; gap: 8px; align-items: center; padding: 2px 0; }
  .keep, .rm { display: inline-flex; flex-direction: row; gap: 4px; align-items: center; }
  .p { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: Consolas, monospace; font-size: 12.5px; user-select: text; }
  .hint { color: var(--text-3); }
  .foot { display: flex; justify-content: space-between; gap: 12px; flex-wrap: wrap; align-items: center; }
  .acts { display: flex; gap: 6px; flex-wrap: wrap; }
</style>
