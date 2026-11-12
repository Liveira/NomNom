<script lang="ts">
  import Dialog from '../../lib/ui/Dialog.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { save as saveFile, open as pickFolder } from '@tauri-apps/plugin-dialog';
  import { formatBytes } from '../../lib/format';
  import { stTools, type StRenameRow, type StRenameRule, type StReportSpec } from '../../lib/ipc';
  import { attributeLabels } from './logic';
  import { storage } from './store.svelte';

  let typed = $state('');
  let readonly = $state(false);
  let newName = $state('');

  // Batch rename
  const blankRule = (): StRenameRule => ({ find: '', replace: '', regex: false, prefix: '', suffix: '', start: 1, step: 1, pad: 0, case: '', include_extension: false });
  let rule = $state<StRenameRule>(blankRule());
  let preview = $state<StRenameRow[]>([]);
  let previewError = $state('');
  let previewSeq = 0;
  /** An emptied number box reads as nothing; the app needs real numbers. */
  function cleanRule(r: StRenameRule): StRenameRule {
    const n = (v: unknown, min: number, fallback: number) => (Number.isFinite(Number(v)) && v !== null && v !== '' ? Math.max(min, Math.floor(Number(v))) : fallback);
    return { ...r, start: n(r.start, 0, 1), step: n(r.step, 1, 1), pad: Math.min(12, n(r.pad, 0, 0)) };
  }

  // ZIP
  let zipDest = $state('');
  let zipName = $state('');

  // Export
  type ReportType = 'tree' | 'top_files' | 'top_folders' | 'extensions' | 'categories' | 'age' | 'duplicates';
  let reportType = $state<ReportType>('tree');
  let format = $state<StReportSpec['format']>('csv');
  let depth = $state(3);
  let topN = $state(100);
  let folderMode = $state<'inclusive' | 'exclusive'>('exclusive');
  let scopeHere = $state(false);
  let columns = $state<Record<string, boolean>>({ name: true, path: true, type: true, size: true, alloc: true, files: true, dirs: true, modified: true, percent: true });
  let exporting = $state(false);
  const COLUMN_LABELS: Record<string, string> = { name: 'Name', path: 'Path', type: 'Type', size: 'Size', alloc: 'On disk', files: 'Files', dirs: 'Folders', modified: 'Modified', percent: '% of folder' };
  const REPORT_LABELS: Record<ReportType, string> = {
    tree: 'Folder tree down to a depth',
    top_files: 'Biggest files',
    top_folders: 'Biggest folders',
    extensions: 'Space by file type',
    categories: 'Space by kind of file',
    age: 'Files by age',
    duplicates: 'Duplicates (search for them first)',
  };
  const hasColumns = $derived(reportType === 'tree' || reportType === 'top_files' || reportType === 'top_folders');

  const dlg = $derived(storage.dialog);

  // Reset the fields whenever a different dialog opens.
  $effect(() => {
    if (dlg?.type === 'confirm') {
      typed = '';
      readonly = false;
    }
    if (dlg?.type === 'rename') newName = dlg.node.name;
    // These keep what the person typed when the same window shows an error, so reset only when it newly opens.
    const opened = dlg?.type !== shown;
    shown = dlg?.type ?? '';
    if (opened && dlg?.type === 'batchrename') {
      rule = blankRule();
      preview = [];
      previewError = '';
    }
    if (opened && dlg?.type === 'zip') {
      zipDest = '';
      zipName = 'Archive.zip';
    }
    if (dlg?.type === 'export') exporting = false;
  });
  let shown = '';

  // The preview follows every change of the rule.
  $effect(() => {
    if (dlg?.type !== 'batchrename' || storage.sessionId == null) return;
    const id = storage.sessionId;
    const nodes = dlg.nodes;
    const r = cleanRule($state.snapshot(rule));
    const mine = ++previewSeq;
    const t = setTimeout(async () => {
      try {
        const rows = await stTools.renamePreview(id, nodes, r);
        if (mine === previewSeq) {
          preview = rows;
          previewError = '';
        }
      } catch (e) {
        if (mine === previewSeq) previewError = String(e);
      }
    }, 150);
    return () => clearTimeout(t);
  });

  const changes = $derived(preview.filter((p) => p.new != null).length);
  const problems = $derived(preview.filter((p) => p.error && p.new == null && p.error !== "The name wouldn't change.").length);

  async function doBatchRename() {
    if (dlg?.type !== 'batchrename' || storage.sessionId == null) return;
    try {
      const plan = await stTools.renamePlan(storage.sessionId, dlg.nodes, cleanRule($state.snapshot(rule)));
      storage.show(plan);
    } catch (e) {
      previewError = String(e);
    }
  }

  async function pickZipFolder() {
    const d = await pickFolder({ directory: true, multiple: false, title: 'Where should the ZIP file go?' });
    if (typeof d === 'string') zipDest = d;
  }

  async function doZip() {
    if (dlg?.type !== 'zip') return;
    const nodes = dlg.nodes;
    const name = zipName.trim().toLowerCase().endsWith('.zip') ? zipName.trim() : `${zipName.trim()}.zip`;
    const plan = await storage.plan('zip', nodes, zipDest, name);
    if (!plan) return;
    if (plan.item_count === 0) {
      storage.dialog = { type: 'zip', nodes, error: plan.refused[0]?.reason ?? "The ZIP file can't be made." };
      return;
    }
    storage.show(plan);
  }

  function spec(): StReportSpec {
    const cols = hasColumns ? Object.keys(columns).filter((k) => columns[k]) : [];
    const base = { format, columns: cols };
    switch (reportType) {
      case 'tree': return { kind: 'tree', depth, ...base };
      case 'top_files': return { kind: 'top_files', n: topN, ...base };
      case 'top_folders': return { kind: 'top_folders', n: topN, mode: folderMode, ...base };
      case 'extensions': return { kind: 'extensions', ...base };
      case 'categories': return { kind: 'categories', ...base };
      case 'age': return { kind: 'age', field: 'modified', ...base };
      case 'duplicates': return { kind: 'duplicates', ...base };
    }
  }

  async function doExport() {
    const id = storage.sessionId;
    if (dlg?.type !== 'export' || id == null || exporting) return;
    if (hasColumns && !Object.values(columns).some(Boolean)) {
      storage.dialog = { type: 'export', error: 'Pick at least one column.' };
      return;
    }
    const path = await saveFile({ title: 'Save the report', defaultPath: `report.${format}`, filters: [{ name: format.toUpperCase(), extensions: [format] }] });
    if (typeof path !== 'string') return;
    exporting = true;
    try {
      const bytes = await stTools.export(id, scopeHere ? storage.current : 0, spec(), path);
      storage.dialog = null;
      storage.say(`Report saved (${formatBytes(bytes)}).`);
    } catch (e) {
      exporting = false;
      storage.dialog = { type: 'export', error: String(e) };
    }
  }

  function close() {
    storage.dialog = null;
  }

  const verbs = {
    recycle: 'Move to the Recycle Bin',
    delete: 'Delete permanently',
    copy: 'Copy',
    move: 'Move',
    rename: 'Rename',
    batch_rename: 'Rename',
    zip: 'Make ZIP file',
  } as const;
  const titles = {
    recycle: 'Move to the Recycle Bin?',
    delete: 'Delete permanently?',
    copy: 'Copy these items?',
    move: 'Move these items?',
    rename: 'Rename?',
    batch_rename: 'Rename these items?',
    zip: 'Add these items to a ZIP file?',
  } as const;

  async function confirm() {
    if (dlg?.type !== 'confirm' || dlg.busy) return;
    storage.dialog = { ...dlg, busy: true, error: '' };
    await storage.runPlan(dlg.plan, true, typed || undefined, readonly);
  }

  const typedOk = $derived(dlg?.type === 'confirm' && (!dlg.plan.needs_typed || typed.trim() === String(dlg.plan.item_count)));
  const when = (t: number | null) => (t == null ? 'unknown' : new Date(t * 1000).toLocaleString());
</script>

{#if dlg?.type === 'confirm'}
  {@const p = dlg.plan}
  {@const permanent = p.kind === 'delete'}
  <Dialog title={titles[p.kind]} onclose={close} locked={dlg.busy}>
    {#if permanent}
      <p class="danger" role="alert"><strong>These items will NOT go to the Recycle Bin.</strong> They can't be brought back by Windows.</p>
    {/if}
    <p>
      {p.item_count.toLocaleString()} {p.item_count === 1 ? 'item' : 'items'}, {p.total_files.toLocaleString()} {p.total_files === 1 ? 'file' : 'files'}, {formatBytes(p.total_size)}
      {#if p.dest}<br />To <span class="mono">{p.dest}</span>{/if}
    </p>
    <ul class="items" aria-label="Items">
      {#each p.items.slice(0, 15) as it (it.path)}
        <li class:warn={it.warn}>
          <span class="mono" title={it.path}>{it.path}{#if it.new_name} → {it.new_name}{/if}{#if it.keep}<br /><span class="hint">identical to the copy kept: {it.keep}</span>{/if}</span>
          <span class="sz tabular">{formatBytes(it.size)}</span>
        </li>
      {/each}
      {#if p.item_count > 15}<li class="hint">and {p.item_count - 15} more</li>{/if}
    </ul>
    {#if p.warnings.length}
      <div class="box warnbox" role="note"><strong>Be careful</strong>{#each p.warnings.slice(0, 5) as w (w)}<div>{w}</div>{/each}</div>
    {/if}
    {#if p.refused.length}
      <div class="box" role="note"><strong>Left out ({p.refused.length})</strong>
        {#each p.refused.slice(0, 5) as r (r.path + r.reason)}<div><span class="mono">{r.path}</span><br /><span class="hint">{r.reason}</span></div>{/each}
      </div>
    {/if}
    {#if permanent}
      <label class="check"><input type="checkbox" bind:checked={readonly} disabled={dlg.busy} /> Also delete read-only files</label>
    {/if}
    {#if p.needs_typed}
      <label class="typed">This is a big or risky delete. Type <strong>{p.item_count}</strong> (the number of items) to confirm:
        <input bind:value={typed} inputmode="numeric" disabled={dlg.busy} aria-label="Type the number of items to confirm" />
      </label>
    {/if}
    {#if dlg.busy && storage.opProgress}
      <ProgressBar value={storage.opProgress.total ? storage.opProgress.done / storage.opProgress.total : null} label="Progress" />
      <p class="hint">{storage.opProgress.done} of {storage.opProgress.total}{storage.opProgress.current ? `: ${storage.opProgress.current}` : ''}</p>
    {/if}
    {#if dlg.error}<p class="error" role="alert">{dlg.error}</p>{/if}
    {#snippet actions()}
      {#if dlg?.type === 'confirm' && dlg.busy}
        <button class="plain" onclick={() => storage.cancelOp()}>Stop</button>
      {:else}
        <button class="plain" onclick={close}>Cancel</button>
        <button class={permanent ? 'plain dangerbtn' : 'primary'} disabled={p.item_count === 0 || !typedOk} onclick={confirm}>{verbs[p.kind]}</button>
      {/if}
    {/snippet}
  </Dialog>
{:else if dlg?.type === 'refused'}
  <Dialog title="Nothing was changed" onclose={close}>
    <p>NomNom didn't do this because:</p>
    <ul class="items">
      {#each dlg.plan.refused.slice(0, 12) as r (r.path + r.reason)}<li><span><span class="mono">{r.path || '(destination)'}</span><br /><span class="hint">{r.reason}</span></span></li>{/each}
    </ul>
    {#snippet actions()}<button class="primary" onclick={close}>OK</button>{/snippet}
  </Dialog>
{:else if dlg?.type === 'rename'}
  <Dialog title="Rename" onclose={close}>
    <label class="typed">New name for <strong>{dlg.node.name}</strong>
      <input bind:value={newName} aria-label="New name" onkeydown={(e) => e.key === 'Enter' && storage.doRename(newName)} />
    </label>
    {#if dlg.error}<p class="error" role="alert">{dlg.error}</p>{/if}
    {#snippet actions()}
      <button class="plain" onclick={close}>Cancel</button>
      <button class="primary" disabled={!newName || newName === (dlg?.type === 'rename' ? dlg.node.name : '')} onclick={() => storage.doRename(newName)}>Rename</button>
    {/snippet}
  </Dialog>
{:else if dlg?.type === 'batchrename'}
  <Dialog title="Rename in batch" onclose={close}>
    <div class="form">
      <label>Find in the name <input bind:value={rule.find} aria-label="Text to find" /></label>
      <label>Replace with <input bind:value={rule.replace} aria-label="Replacement" /></label>
      <label class="check"><input type="checkbox" bind:checked={rule.regex} /> Find is a regular expression</label>
      <label>Add before <input bind:value={rule.prefix} aria-label="Text to add before" /></label>
      <label>Add after <input bind:value={rule.suffix} aria-label="Text to add after" /></label>
      <div class="row3">
        <label>Number starts at <input type="number" min="0" bind:value={rule.start} /></label>
        <label>Step <input type="number" min="1" bind:value={rule.step} /></label>
        <label>Digits <input type="number" min="0" max="12" bind:value={rule.pad} /></label>
      </div>
      <p class="hint">Write <span class="mono">{'{n}'}</span> where the number should go, for example <span class="mono">Holiday {'{n}'}</span>.</p>
      <label>Letters
        <select bind:value={rule.case}>
          <option value="">Keep as they are</option>
          <option value="lower">all lowercase</option>
          <option value="upper">ALL UPPERCASE</option>
        </select>
      </label>
      <label class="check"><input type="checkbox" bind:checked={rule.include_extension} /> Change the file type (extension) too</label>
    </div>
    <p class="count">{changes} of {preview.length} {preview.length === 1 ? 'name' : 'names'} would change{problems ? `, ${problems} can't be used` : ''}.</p>
    {#if previewError}<p class="error" role="alert">{previewError}</p>{/if}
    <ul class="items" aria-label="Preview">
      {#each preview.slice(0, 200) as p, i (i)}
        <li class:warn={!!p.error && p.new == null}>
          <span class="mono">{p.old}{#if p.new} → {p.new}{/if}{#if p.error}<br /><span class="hint">{p.error}</span>{/if}</span>
        </li>
      {/each}
      {#if preview.length > 200}<li class="hint">and {preview.length - 200} more</li>{/if}
    </ul>
    {#snippet actions()}
      <button class="plain" onclick={close}>Cancel</button>
      <button class="primary" disabled={changes === 0} onclick={doBatchRename}>Review…</button>
    {/snippet}
  </Dialog>
{:else if dlg?.type === 'zip'}
  <Dialog title="Add to a ZIP file" onclose={close}>
    <p>{dlg.nodes.length.toLocaleString()} {dlg.nodes.length === 1 ? 'item goes' : 'items go'} into a new ZIP file. The originals stay where they are.</p>
    <div class="form">
      <label>Save in
        <span class="inline"><input class="grow" bind:value={zipDest} aria-label="Folder for the ZIP file" /> <button class="plain" onclick={pickZipFolder}>Choose…</button></span>
      </label>
      <label>File name <input bind:value={zipName} aria-label="ZIP file name" /></label>
    </div>
    {#if dlg.error}<p class="error" role="alert">{dlg.error}</p>{/if}
    {#snippet actions()}
      <button class="plain" onclick={close}>Cancel</button>
      <button class="primary" disabled={!zipDest.trim() || !zipName.trim()} onclick={doZip}>Review…</button>
    {/snippet}
  </Dialog>
{:else if dlg?.type === 'export'}
  <Dialog title="Save a report" onclose={close} locked={exporting}>
    <div class="form">
      <label>Report
        <select bind:value={reportType}>
          {#each Object.entries(REPORT_LABELS) as [k, label] (k)}<option value={k}>{label}</option>{/each}
        </select>
      </label>
      {#if reportType === 'tree'}
        <label>Levels deep <input type="number" min="1" max="12" bind:value={depth} /></label>
      {/if}
      {#if reportType === 'top_files' || reportType === 'top_folders'}
        <label>How many <input type="number" min="1" max="100000" bind:value={topN} /></label>
      {/if}
      {#if reportType === 'top_folders'}
        <label>Size of a folder
          <select bind:value={folderMode}><option value="exclusive">Files directly inside</option><option value="inclusive">Everything inside (overlapping)</option></select>
        </label>
      {/if}
      {#if reportType !== 'duplicates'}
        <label class="check"><input type="checkbox" bind:checked={scopeHere} /> Only the folder I'm looking at</label>
      {/if}
      <label>File format
        <select bind:value={format}>
          <option value="csv">CSV (spreadsheet text)</option>
          <option value="xlsx">Excel (XLSX)</option>
          <option value="json">JSON</option>
          <option value="html">Web page (HTML)</option>
          <option value="pdf">PDF</option>
        </select>
      </label>
      {#if hasColumns}
        <fieldset>
          <legend>Columns</legend>
          <div class="cols">
            {#each Object.entries(COLUMN_LABELS) as [k, label] (k)}<label class="check"><input type="checkbox" bind:checked={columns[k]} /> {label}</label>{/each}
          </div>
        </fieldset>
      {/if}
    </div>
    <p class="hint">Sizes are what the scan found. The report says so, and it never replaces an existing file.</p>
    {#if dlg.error}<p class="error" role="alert">{dlg.error}</p>{/if}
    {#snippet actions()}
      <button class="plain" onclick={close} disabled={exporting}>Cancel</button>
      <button class="primary" disabled={exporting} onclick={doExport}>{exporting ? 'Saving…' : 'Choose where to save…'}</button>
    {/snippet}
  </Dialog>
{:else if dlg?.type === 'result'}
  {@const r = dlg.report.results}
  {@const done = r.filter((x) => x.status === 'done').length}
  <Dialog title="Finished with some problems" onclose={close}>
    <p>{done} done, {r.filter((x) => x.status === 'failed').length} failed, {r.filter((x) => x.status === 'skipped').length} skipped.{dlg.report.cancelled ? ' You stopped it before the end.' : ''}</p>
    <ul class="items">
      {#each r.filter((x) => x.status !== 'done').slice(0, 30) as x (x.path)}
        <li><span><span class="mono" title={x.path}>{x.path}</span><br /><span class="hint">{x.status === 'failed' ? 'Failed' : 'Skipped'}: {x.message}</span></span></li>
      {/each}
    </ul>
    {#snippet actions()}<button class="primary" onclick={close}>OK</button>{/snippet}
  </Dialog>
{:else if dlg?.type === 'props'}
  {@const n = dlg.node}
  <Dialog title="Properties" onclose={close}>
    <dl>
      <dt>Name</dt><dd>{n.name}</dd>
      <dt>Location</dt><dd class="mono">{dlg.path}</dd>
      <dt>Type</dt><dd>{n.is_link ? 'Link (not followed)' : n.is_dir ? 'Folder' : 'File'}</dd>
      <dt>Size</dt><dd class="tabular">{formatBytes(n.size)} ({n.size.toLocaleString()} bytes)</dd>
      <dt>On disk</dt><dd class="tabular">{formatBytes(n.alloc)}</dd>
      {#if n.is_dir}<dt>Contains</dt><dd class="tabular">{n.files.toLocaleString()} files, {n.dirs.toLocaleString()} folders</dd>{/if}
      <dt>Created</dt><dd>{when(n.ctime)}</dd>
      <dt>Modified</dt><dd>{when(n.mtime)}</dd>
      <dt>Last opened</dt><dd>{when(n.atime)} <span class="hint">(Windows may not keep this up to date)</span></dd>
      <dt>Attributes</dt><dd>{attributeLabels(n.attrs, n.flags).join(', ') || 'None'}</dd>
    </dl>
    {#snippet actions()}<button class="primary" onclick={close}>Close</button>{/snippet}
  </Dialog>
{/if}

<style>
  p { margin: 0 0 8px; }
  .danger { color: var(--danger); }
  .mono { font-family: Consolas, monospace; word-break: break-all; user-select: text; font-size: 12.5px; }
  .items { list-style: none; margin: 0 0 10px; padding: 0; max-height: 200px; overflow: auto; border: 1px solid var(--border); border-radius: var(--radius-s); }
  .items li { display: flex; justify-content: space-between; gap: 12px; padding: 4px 8px; border-bottom: 1px solid var(--border); }
  .items li:last-child { border-bottom: 0; }
  .items li.warn { background: var(--surface-2); }
  .sz { flex: none; color: var(--text-2); }
  .hint { color: var(--text-3); font-size: 12.5px; }
  .box { border: 1px solid var(--border-strong); border-radius: var(--radius-s); padding: 8px 10px; margin-bottom: 10px; display: grid; gap: 4px; font-size: 13px; }
  .warnbox { border-color: var(--danger); }
  .check, .typed { display: grid; gap: 6px; margin-bottom: 8px; }
  .check { display: flex; gap: 8px; align-items: center; }
  .typed input { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .error { color: var(--danger); }
  .form { display: grid; gap: 8px; margin-bottom: 8px; }
  .form label { display: grid; gap: 3px; color: var(--text-2); font-size: 12.5px; }
  .form label.check { display: flex; }
  .form input:not([type='checkbox']), .form select { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .row3 { display: grid; grid-template-columns: repeat(3, 1fr); gap: 8px; }
  .inline { display: flex; gap: 6px; }
  .grow { flex: 1; min-width: 0; }
  .count { color: var(--text-2); }
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-s); padding: 6px 10px; }
  legend { color: var(--text-2); font-size: 12.5px; padding: 0 4px; }
  .cols { display: grid; grid-template-columns: repeat(3, 1fr); gap: 4px 12px; }
  dl { display: grid; grid-template-columns: 110px 1fr; gap: 6px 12px; margin: 0; }
  dt { color: var(--text-3); }
  dd { margin: 0; min-width: 0; }
  button { padding: 7px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.dangerbtn { border-color: var(--danger); color: var(--danger); font-weight: 600; }
</style>
