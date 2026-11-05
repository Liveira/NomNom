<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { app } from '../../app/state.svelte';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import {
    onOrganizerChanged,
    orgIpc,
    type OrgBatch,
    type OrgReport,
    type OrgScan,
    type OrgUndoReport,
  } from '../../lib/ipc';
  import { batchStatus, batchTitle, canUndo, fileName, groupMoves, groupSkips, headline, plural, resultLine, undoLine } from './summary';

  let source = $state<string | null>(null); // null means the Downloads folder
  let defaultSource = $state('');
  let depth = $state(app.settings?.organizer.look_inside_folders ?? 0);
  let phase = $state<'idle' | 'scanning' | 'ready' | 'running' | 'done'>('idle');
  let scan = $state<OrgScan | null>(null);
  let report = $state<OrgReport | null>(null);
  let undone = $state<OrgUndoReport | null>(null);
  let error = $state('');
  let allowCross = $state(false);
  let history = $state<OrgBatch[]>([]);
  let undoing = $state<string | null>(null);

  const groups = $derived(scan ? groupMoves(scan.moves) : []);
  const skipGroups = $derived(scan ? groupSkips(scan.skips) : []);
  const auto = $derived(app.settings?.organizer.auto_enabled ?? false);
  const shownSource = $derived(source ?? defaultSource);
  const blockedByDrive = $derived((scan?.cross_volume_count ?? 0) > 0 && !allowCross);

  async function loadHistory() {
    history = await orgIpc.history();
  }

  $effect(() => {
    void orgIpc.defaults().then((d) => (defaultSource = d.source));
    void loadHistory();
    const unlisten = onOrganizerChanged(() => void loadHistory());
    return () => void unlisten.then((f) => f());
  });

  async function chooseFolder() {
    const picked = await open({ directory: true, multiple: false, defaultPath: shownSource });
    if (typeof picked === 'string') {
      source = picked;
      reset();
    }
  }

  function reset() {
    phase = 'idle';
    scan = null;
    report = null;
    undone = null;
    error = '';
    allowCross = false;
  }

  async function doScan() {
    reset();
    phase = 'scanning';
    try {
      scan = await orgIpc.scan(source, depth);
      phase = 'ready';
    } catch (e) {
      error = String(e);
      phase = 'idle';
    }
  }

  async function doMove() {
    if (!scan) return;
    phase = 'running';
    error = '';
    try {
      // Only the id of the preview goes back; the app moves exactly what it showed.
      report = await orgIpc.execute(scan.plan_id, allowCross);
      phase = 'done';
    } catch (e) {
      error = String(e);
      phase = 'idle';
    }
    await loadHistory();
  }

  async function doUndo(id: string) {
    undoing = id;
    error = '';
    try {
      undone = await orgIpc.undo(id, false);
      if (report?.batch_id === id) report = { ...report, moved: 0 };
    } catch (e) {
      error = String(e);
    } finally {
      undoing = null;
      await loadHistory();
    }
  }

  const when = (secs: number) => new Date(secs * 1000).toLocaleString([], { dateStyle: 'medium', timeStyle: 'short' });
</script>

<h1>File Organizer</h1>

<section class="card note">
  <p>
    NomNom looks first and shows you what it would do. Folders with programs, installers or code are
    never opened. Nothing is ever replaced, and every move can be undone.
  </p>
  <p class="auto">
    Automatic organizing of new downloads is <strong>{auto ? 'on' : 'off'}</strong>.
    <button class="link" onclick={() => app.go('settings')}>Change it in Settings</button>
  </p>
</section>

<section class="card">
  <div class="row">
    <div>
      <div class="label">Folder to organize</div>
      <div class="hint path">{shownSource}</div>
    </div>
    <div class="buttons">
      <button class="plain" onclick={chooseFolder} disabled={phase === 'scanning' || phase === 'running'}>Choose folder</button>
      {#if source}<button class="plain" onclick={() => { source = null; reset(); }}>Use Downloads</button>{/if}
    </div>
  </div>
  <div class="row">
    <label class="label" for="depth">Look inside folders</label>
    <select id="depth" bind:value={depth} onchange={reset}>
      <option value={0}>No, only the files in it</option>
      <option value={1}>1 level down</option>
      <option value={2}>2 levels down</option>
      <option value={3}>3 levels down</option>
    </select>
  </div>
  <div class="row end">
    <button class="primary" onclick={doScan} disabled={phase === 'scanning' || phase === 'running'}>
      {phase === 'scanning' ? 'Looking…' : scan ? 'Scan again' : 'Scan'}
    </button>
  </div>
</section>

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if scan && (phase === 'ready' || phase === 'running')}
  <section class="card" aria-live="polite">
    <h2>{headline(scan)}</h2>
    <p class="hint">Files would go to <span class="mono">{scan.dest_root}</span></p>

    {#if scan.moves.length === 0}
      <div class="empty"><CatNap width={96} /><p>Nothing to move right now.</p></div>
    {/if}

    {#each groups as g (g.category)}
      <details class="group" open={groups.length <= 2}>
        <summary><strong>{g.category}</strong> <span class="count">{plural(g.items.length, 'file', 'files')}</span></summary>
        <ul>
          {#each g.items.slice(0, 200) as m (m.src)}
            <li>
              <span class="name" title={m.src}>{fileName(m.src)}</span>
              {#if m.cross_volume}<span class="tag">another drive</span>{/if}
            </li>
          {/each}
          {#if g.items.length > 200}<li class="more">and {g.items.length - 200} more</li>{/if}
        </ul>
      </details>
    {/each}

    {#if scan.cross_volume_count > 0}
      <label class="check">
        <input type="checkbox" bind:checked={allowCross} />
        <span>
          {plural(scan.cross_volume_count, 'file is', 'files are')} on another drive. Copy {scan.cross_volume_count === 1 ? 'it' : 'them'},
          check the copy, then remove the original.
        </span>
      </label>
    {/if}

    {#if scan.moves.length > 0}
      <div class="row end">
        {#if phase === 'running'}
          <button class="plain" onclick={() => orgIpc.cancel()}>Stop</button>
          <span class="hint">Moving files…</span>
        {:else}
          <button class="primary" onclick={doMove} disabled={blockedByDrive && scan.moves.every((m) => m.cross_volume)}>
            Move {plural(scan.moves.length - (blockedByDrive ? scan.cross_volume_count : 0), 'file', 'files')}
          </button>
        {/if}
      </div>
    {/if}
  </section>

  {#if skipGroups.length > 0}
    <section class="card">
      <h2>Left alone</h2>
      <p class="hint">These stay exactly where they are.</p>
      {#each skipGroups as g (g.code)}
        <details class="group">
          <summary><strong>{g.title}</strong> <span class="count">{g.items.length}</span></summary>
          <ul>
            {#each g.items.slice(0, 100) as s (s.path)}
              <li>
                <span class="name" title={s.path}>{fileName(s.path)}{s.is_dir ? '\\' : ''}</span>
                <span class="why">{s.message}</span>
              </li>
            {/each}
            {#if g.items.length > 100}<li class="more">and {g.items.length - 100} more</li>{/if}
          </ul>
        </details>
      {/each}
    </section>
  {/if}
{/if}

{#if phase === 'done' && report}
  <section class="card done" aria-live="polite">
    <h2>{undone ? undoLine(undone) : resultLine(report)}</h2>
    {#each report.failed.slice(0, 20) as f (f.path)}
      <p class="hint"><span class="name">{fileName(f.path)}</span>: {f.message}</p>
    {/each}
    <div class="row end">
      {#if report.moved > 0}
        <button class="plain" disabled={undoing != null} onclick={() => doUndo(report!.batch_id)}>Undo</button>
      {/if}
      <button class="primary" onclick={doScan}>Scan again</button>
    </div>
  </section>
{/if}

<h2 class="section">History</h2>
{#if history.length === 0}
  <div class="empty"><CatNap width={96} /><p>Nothing organized yet.</p></div>
{:else}
  <ul class="history">
    {#each history as b (b.id)}
      <li class="card item">
        <div>
          <div class="label">{batchTitle(b)}</div>
          <div class="hint">{when(b.created_at)} · {batchStatus(b)}</div>
        </div>
        {#if canUndo(b)}
          <button class="plain" disabled={undoing != null} onclick={() => doUndo(b.id)}>{undoing === b.id ? 'Putting back…' : 'Undo'}</button>
        {/if}
      </li>
    {/each}
  </ul>
  {#if undone && phase !== 'done'}<p class="ok" role="status">{undoLine(undone)}</p>{/if}
{/if}

<style>
  h1 { margin-bottom: 18px; }
  h2 { font-size: 16px; margin: 0 0 6px; }
  h2.section { margin: 26px 0 10px; }
  .card { max-width: 760px; margin-bottom: 14px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 14px 16px; }
  .note p { margin: 0; color: var(--text-2); }
  .note .auto { margin-top: 8px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 18px; }
  .row + .row { margin-top: 12px; }
  .row.end { justify-content: flex-end; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 2px 0 0; }
  .path, .mono { font-family: Consolas, monospace; word-break: break-all; user-select: text; }
  .buttons { display: flex; gap: 8px; flex: none; }
  button, select { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.link { border: 0; background: none; padding: 0 4px; color: var(--accent-text); text-decoration: underline; }
  .error { color: var(--danger); max-width: 760px; }
  .ok { color: var(--ok); }
  .group { margin-top: 8px; border-top: 1px solid var(--border); padding-top: 8px; }
  summary { cursor: pointer; }
  .count { color: var(--text-3); margin-left: 8px; }
  ul { list-style: none; margin: 8px 0 0; padding: 0; max-height: 260px; overflow: auto; display: grid; gap: 2px; }
  li { display: flex; justify-content: space-between; gap: 12px; padding: 3px 4px; border-radius: 6px; }
  li:hover { background: var(--surface-2); }
  li.more { color: var(--text-3); }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
  .why { color: var(--text-3); font-size: 12px; flex: none; max-width: 55%; text-align: right; }
  .tag { font-size: 11.5px; color: var(--info); flex: none; }
  .check { display: flex; gap: 10px; align-items: flex-start; margin-top: 12px; color: var(--text-2); }
  .empty { display: flex; align-items: center; gap: 14px; color: var(--text-3); }
  .empty p { margin: 0; }
  .history { list-style: none; padding: 0; margin: 0; max-height: none; overflow: visible; }
  .history .item { display: flex; align-items: center; justify-content: space-between; gap: 14px; padding: 10px 14px; margin-bottom: 8px; }
  .history .item:hover { background: var(--surface-1); }
</style>
