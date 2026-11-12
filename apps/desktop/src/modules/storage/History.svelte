<script lang="ts">
  import { open as pickFolder, save as saveFile } from '@tauri-apps/plugin-dialog';
  import { app } from '../../app/state.svelte';
  import { formatBytes } from '../../lib/format';
  import {
    onStorageSchedule,
    onStorageSnapshots,
    stHistory,
    type StChangeRow,
    type StCompareView,
    type StRunNote,
    type StSnapshot,
    type StTrendPoint,
  } from '../../lib/ipc';
  import { CHANGE_LABELS, dateTimeText, dayText, errorsNote, scheduleLabel, scopeNote, signedBytes, snapshotName, summaryLine, trendBars } from './history';
  import { storage } from './store.svelte';

  let snapshots = $state<StSnapshot[]>([]);
  let picked = $state<number[]>([]);
  let label = $state('');
  let error = $state('');
  let view = $state<StCompareView | null>(null);
  let comparing = $state(false);
  let trend = $state<StTrendPoint[]>([]);
  let trendFolder = $state('');
  let format = $state<'csv' | 'xlsx' | 'json' | 'html' | 'pdf'>('csv');
  let removing = $state<number | null>(null);
  let sched = $state<{ running: string | null; notes: StRunNote[] }>({ running: null, notes: [] });
  let saving = $state(false);

  const p = $derived(storage.progress);
  const root = $derived(p?.root ?? '');
  const finished = $derived(p?.state === 'done');
  const cfg = $derived(app.settings?.storage);
  const bars = $derived(trendBars(trend));

  async function reload() {
    if (!root) return;
    try {
      snapshots = await stHistory.list(root);
      picked = picked.filter((id) => snapshots.some((s) => s.id === id));
      trend = await stHistory.trend(root, trendFolder);
    } catch (e) {
      error = String(e);
    }
  }
  async function reloadStatus() {
    sched = await stHistory.scheduleStatus().catch(() => sched);
  }

  $effect(() => {
    void root;
    view = null;
    trendFolder = '';
    picked = [];
    void reload();
    void reloadStatus();
  });
  $effect(() => {
    const offs: (() => void)[] = [];
    let gone = false;
    void onStorageSnapshots(() => void reload()).then((f) => (gone ? f() : offs.push(f)));
    void onStorageSchedule(() => void reloadStatus()).then((f) => (gone ? f() : offs.push(f)));
    return () => {
      gone = true;
      offs.forEach((f) => f());
    };
  });

  async function saveSnapshot() {
    if (storage.sessionId == null) return;
    saving = true;
    error = '';
    try {
      await stHistory.save(storage.sessionId, label);
      label = '';
      storage.say('Snapshot saved.');
      await reload();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  function toggle(id: number) {
    picked = picked.includes(id) ? picked.filter((x) => x !== id) : picked.length >= 2 ? [picked[1], id] : [...picked, id];
  }

  async function compare() {
    if (picked.length === 0) return;
    comparing = true;
    error = '';
    try {
      const [a, b] = picked;
      view = await stHistory.compare(a, b ?? null, b == null ? storage.sessionId : null, 500);
    } catch (e) {
      error = String(e);
      view = null;
    } finally {
      comparing = false;
    }
  }

  async function remove(id: number) {
    if (removing !== id) {
      removing = id;
      return;
    }
    removing = null;
    try {
      await stHistory.remove(id);
      if (view && (view.before.id === id || view.after?.id === id)) view = null;
      await reload();
    } catch (e) {
      error = String(e);
    }
  }

  async function showTrend(folder: string) {
    trendFolder = folder;
    try {
      trend = await stHistory.trend(root, folder);
    } catch (e) {
      error = String(e);
    }
  }

  async function exportIt() {
    if (!view || picked.length === 0) return;
    const path = await saveFile({ title: 'Save the comparison', defaultPath: `changes.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] });
    if (typeof path !== 'string') return;
    try {
      const [a, b] = picked;
      const n = await stHistory.exportComparison(a, b ?? null, b == null ? storage.sessionId : null, format, path);
      storage.say(`Comparison saved (${formatBytes(n)}).`);
    } catch (e) {
      error = String(e);
    }
  }

  // ---- schedule ----------------------------------------------------------------------------------
  async function setSchedule(patch: Parameters<typeof app.update>[0]) {
    error = '';
    try {
      await app.update(patch);
      await reloadStatus();
    } catch (e) {
      error = String(e);
    }
  }
  async function addFolder() {
    const d = await pickFolder({ directory: true, multiple: false, title: 'Scan this folder on a schedule' });
    if (typeof d === 'string' && cfg) await setSchedule({ storageRoots: [...cfg.schedule_roots, d] });
  }
  async function removeFolder(r: string) {
    if (cfg) await setSchedule({ storageRoots: cfg.schedule_roots.filter((x) => x !== r) });
  }

  const nameOf = (m: StSnapshot | null) => (m ? snapshotName(m) : 'the scan open now');
  const rowText = (r: StChangeRow) => `${CHANGE_LABELS[r.change]}`;
  const pickedNames = $derived(picked.map((id) => snapshots.find((s) => s.id === id)).filter((s): s is StSnapshot => !!s));
</script>

<div class="wrap">
  {#if error}<p class="error" role="alert">{error} <button class="link" onclick={() => (error = '')}>Dismiss</button></p>{/if}

  <section aria-label="Save a snapshot">
    <h3>Snapshots of {root}</h3>
    <p class="hint">A snapshot keeps the size of every folder down to {cfg?.snapshot_depth ?? 6} levels, and the name of every file of {cfg?.big_file_mb ?? 50} MB or more, as they are today. Use it later to see what grew. It is a record of one moment, not the current state.</p>
    <div class="line">
      <input bind:value={label} placeholder="Name (optional)" aria-label="Name for the snapshot" maxlength="80" onkeydown={(e) => e.key === 'Enter' && finished && saveSnapshot()} />
      <button class="primary" disabled={!finished || saving} onclick={saveSnapshot}>{saving ? 'Saving…' : 'Save a snapshot of this scan'}</button>
      {#if !finished}<span class="hint">Wait for the scan to finish first.</span>{/if}
    </div>
  </section>

  <section aria-label="Saved snapshots">
    {#if snapshots.length === 0}
      <p class="hint">No snapshots of this folder yet.</p>
    {:else}
      <div class="list" role="list">
        {#each [...snapshots].reverse() as s (s.id)}
          <div class="snap" role="listitem">
            <label class="pick"><input type="checkbox" checked={picked.includes(s.id)} onchange={() => toggle(s.id)} aria-label={`Select the snapshot from ${dateTimeText(s.created_at)}`} />
              <span><strong>{s.label || dateTimeText(s.created_at)}</strong>{s.label ? ` · ${dateTimeText(s.created_at)}` : ''}</span></label>
            <span class="tabular">{formatBytes(s.bytes)}</span>
            <span class="tabular hint">{s.files.toLocaleString()} files</span>
            <span class="hint">{s.source === 'scheduled' ? 'Scheduled' : 'Saved by you'}{errorsNote(s) ? ` · ${errorsNote(s)}` : ''}</span>
            <button class="plain small" onclick={() => remove(s.id)}>{removing === s.id ? 'Really delete?' : 'Delete'}</button>
          </div>
        {/each}
      </div>
      <div class="line">
        <button class="primary" disabled={picked.length === 0 || comparing || (picked.length === 1 && !finished)} onclick={compare}>
          {picked.length === 2 ? 'Compare these two' : 'Compare with the open scan'}
        </button>
        <span class="hint">{picked.length === 0 ? 'Tick one snapshot to compare it with the open scan, or two to compare them.' : picked.length === 1 && !finished ? 'The open scan must be finished.' : ''}</span>
      </div>
    {/if}
  </section>

  {#if view}
    {@const c = view.comparison}
    <section aria-label="What changed">
      <h3>What changed</h3>
      <p class="sum" class:up={c.delta > 0} class:down={c.delta < 0}>{summaryLine(c, nameOf(view.before), nameOf(view.after))}</p>
      <p class="hint">{scopeNote(c)}</p>
      {#if view.before.errors > 0 || (view.after?.errors ?? 0) > 0}
        <p class="hint">Some places couldn't be read in {view.before.errors > 0 ? 'the older' : 'the newer'} scan, so a change there may be missing or wrong.</p>
      {/if}
      <div class="line">
        <select bind:value={format} aria-label="File format"><option value="csv">CSV</option><option value="xlsx">Excel (XLSX)</option><option value="json">JSON</option><option value="html">Web page (HTML)</option><option value="pdf">PDF</option></select>
        <button class="plain" onclick={exportIt}>Save this list…</button>
      </div>
      <h4>Folders ({c.dirs_changed.toLocaleString()} changed{c.dirs_changed > c.dirs.length ? `, biggest ${c.dirs.length.toLocaleString()} shown` : ''})</h4>
      {#if c.dirs.length === 0}<p class="hint">No folder changed in size.</p>{:else}
        <div class="rows" role="table" aria-label="Folders that changed">
          {#each c.dirs as r (r.path)}
            <button class="row" role="row" onclick={() => showTrend(r.path)} title="Show this folder over time">
              <span class="tag {r.change}">{rowText(r)}</span>
              <span class="p" title={r.path}>{r.path}</span>
              <span class="tabular n">{r.before == null ? '' : formatBytes(r.before)}</span>
              <span class="tabular n">{r.after == null ? '' : formatBytes(r.after)}</span>
              <span class="tabular n d" class:up={r.delta > 0} class:down={r.delta < 0}>{signedBytes(r.delta)}</span>
            </button>
          {/each}
        </div>
      {/if}
      <h4>Big files ({c.files_changed.toLocaleString()} changed)</h4>
      {#if c.files.length === 0}<p class="hint">No big file appeared, disappeared or changed size.</p>{:else}
        <div class="rows" role="table" aria-label="Big files that changed">
          {#each c.files as r (r.path)}
            <div class="row static" role="row">
              <span class="tag {r.change}">{rowText(r)}</span>
              <span class="p" title={r.path}>{r.path}</span>
              <span class="tabular n">{r.before == null ? '' : formatBytes(r.before)}</span>
              <span class="tabular n">{r.after == null ? '' : formatBytes(r.after)}</span>
              <span class="tabular n d" class:up={r.delta > 0} class:down={r.delta < 0}>{signedBytes(r.delta)}</span>
            </div>
          {/each}
        </div>
      {/if}
    </section>
  {/if}

  {#if snapshots.length > 1}
    <section aria-label="Size over time">
      <h3>Size over time <span class="hint">{trendFolder ? trendFolder : 'the whole scan'}</span>
        {#if trendFolder}<button class="link" onclick={() => showTrend('')}>Show the whole scan</button>{/if}</h3>
      <div class="bars" role="img" aria-label={`Size of ${trendFolder || 'the whole scan'} at each snapshot`}>
        {#each trend as t, i (t.id)}
          <div class="col" title={`${dayText(t.created_at)}: ${t.size == null ? 'not kept in this snapshot' : formatBytes(t.size)}`}>
            <span class="val tabular">{t.size == null ? 'n/a' : formatBytes(t.size)}</span>
            <span class="bar" class:missing={bars[i].missing} style:height="{Math.max(2, bars[i].height * 100)}px"></span>
            <span class="when hint">{dayText(t.created_at)}</span>
          </div>
        {/each}
      </div>
    </section>
  {/if}

  <section aria-label="Scheduled scans">
    <h3>Scan on a schedule</h3>
    <p class="hint">NomNom can scan folders for you while it is running, and keep a snapshot of each. It only reads folder information and never deletes or moves anything. It waits while you are scanning something yourself.</p>
    <div class="line">
      <label>When
        <select value={cfg?.schedule ?? 'off'} onchange={(e) => setSchedule({ storageSchedule: e.currentTarget.value as 'off' })}>
          {#each ['off', 'on_start', 'daily', 'weekly'] as s (s)}<option value={s}>{scheduleLabel(s)}</option>{/each}
        </select>
      </label>
      <label>Keep the newest
        <input class="num" type="number" min="1" max="100" value={cfg?.keep_scheduled ?? 12} onchange={(e) => setSchedule({ storageKeep: Number(e.currentTarget.value) || 12 })} aria-label="How many scheduled snapshots to keep for each folder" />
      </label>
      <label>Folder levels
        <input class="num" type="number" min="1" max="12" value={cfg?.snapshot_depth ?? 6} onchange={(e) => setSchedule({ storageDepth: Number(e.currentTarget.value) || 6 })} />
      </label>
      <label>Name files from
        <span class="inline"><input class="num" type="number" min="1" value={cfg?.big_file_mb ?? 50} onchange={(e) => setSchedule({ storageBigMb: Number(e.currentTarget.value) || 50 })} /> MB</span>
      </label>
    </div>
    <div class="folders" role="list" aria-label="Folders scanned on a schedule">
      {#each cfg?.schedule_roots ?? [] as r (r)}
        <div class="frow" role="listitem"><span class="p">{r}</span><button class="plain small" onclick={() => removeFolder(r)} aria-label={`Stop scanning ${r}`}>Remove</button></div>
      {:else}
        <p class="hint">No folders chosen. The schedule does nothing until you add one.</p>
      {/each}
    </div>
    <div class="line">
      <button class="plain" onclick={addFolder} disabled={(cfg?.schedule_roots.length ?? 0) >= 10}>Add a folder or drive…</button>
      {#if root && !(cfg?.schedule_roots ?? []).some((r) => r.toLowerCase() === root.toLowerCase())}
        <button class="plain" onclick={() => cfg && setSchedule({ storageRoots: [...cfg.schedule_roots, root] })}>Add {root}</button>
      {/if}
    </div>
    {#if sched.running}<p class="partial" role="status">Scanning {sched.running} now…</p>{/if}
    {#each [...sched.notes].reverse().slice(0, 5) as n (n.at + n.root)}
      <p class="hint" class:bad={!n.ok}>{dateTimeText(n.at)} · {n.root} · {n.ok ? 'Snapshot saved.' : n.message}</p>
    {/each}
  </section>
</div>

<style>
  .wrap { height: 100%; overflow: auto; display: grid; align-content: start; gap: 12px; padding-right: 4px; }
  section { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 12px 14px; display: grid; gap: 8px; min-width: 0; }
  h3 { font-size: 14px; display: flex; gap: 10px; align-items: baseline; flex-wrap: wrap; }
  h4 { font-size: 13px; margin-top: 6px; color: var(--text-2); }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 0; }
  .hint.bad { color: var(--danger); }
  .error { color: var(--danger); margin: 0; }
  .partial { color: var(--info); margin: 0; }
  .line { display: flex; flex-wrap: wrap; gap: 10px; align-items: end; }
  label { display: grid; gap: 3px; color: var(--text-2); font-size: 12.5px; }
  .inline { display: inline-flex; gap: 6px; align-items: center; }
  .num { width: 72px; }
  input:not([type='checkbox']), select { padding: 5px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  button { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.small { padding: 3px 10px; }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; padding: 0; font-size: 12.5px; }
  .list { display: grid; gap: 2px; }
  .snap { display: grid; grid-template-columns: minmax(0, 1.4fr) auto auto minmax(0, 1fr) auto; gap: 4px 14px; min-width: 0; align-items: center; padding: 4px 6px; border-radius: 6px; }
  .snap:hover { background: var(--surface-2); }
  .pick { display: flex; flex-direction: row; gap: 8px; align-items: center; color: var(--text); font-size: 13px; }
  .sum { margin: 0; font-weight: 600; }
  .up { color: var(--danger); }
  .down { color: var(--ok); }
  .rows { display: grid; gap: 1px; max-height: 280px; overflow: auto; border: 1px solid var(--border); border-radius: var(--radius-s); }
  .row { display: grid; grid-template-columns: 70px minmax(120px, 1fr) 86px 86px 100px; gap: 10px; align-items: center; padding: 4px 8px; border: 0; border-radius: 0; background: none; text-align: left; width: 100%; }
  button.row:hover { background: var(--surface-2); }
  .row.static { cursor: default; }
  .p { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-family: Consolas, monospace; font-size: 12.5px; user-select: text; }
  .n { text-align: right; color: var(--text-2); }
  .d { font-weight: 600; }
  .tag { font-size: 11.5px; padding: 1px 6px; border-radius: 4px; background: var(--surface-3); width: fit-content; }
  .bars { display: flex; gap: 10px; align-items: end; overflow-x: auto; padding: 6px 2px; min-height: 150px; }
  .col { display: grid; justify-items: center; gap: 4px; min-width: 76px; }
  .bar { width: 36px; background: var(--accent); border-radius: 4px 4px 0 0; display: block; }
  .bar.missing { background: var(--surface-3); border: 1px dashed var(--border-strong); }
  .val { font-size: 12px; color: var(--text-2); }
  .when { font-size: 11.5px; }
  .folders { display: grid; gap: 4px; }
  .frow { display: flex; justify-content: space-between; gap: 10px; align-items: center; }
</style>
