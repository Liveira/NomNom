<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { formatBytes } from '../../lib/format';
  import { backupIpc, type BackupPlan, type BackupRestoreReport, type BackupTree, type BackupVersion } from '../../lib/ipc';
  import Dialog from '../../lib/ui/Dialog.svelte';

  let { plan, version, onclose }: { plan: BackupPlan; version: BackupVersion; onclose: () => void } = $props();

  let path = $state<string[]>([]);
  let tree = $state<BackupTree | null>(null);
  let picked = $state<Set<string>>(new Set());
  let target = $state('');
  let busy = $state(false);
  let error = $state('');
  let report = $state<BackupRestoreReport | null>(null);
  let checking = $state(false);
  let check = $state('');

  const prefix = $derived(path.join('/'));
  const when = $derived(new Date(version.started_at * 1000).toLocaleString());

  async function load() {
    try {
      tree = await backupIpc.tree(plan.id, version.name, prefix);
    } catch (e) {
      error = String(e);
    }
  }
  void load();

  function enter(name: string) {
    path = [...path, name];
    void load();
  }
  function up(to: number) {
    path = path.slice(0, to);
    void load();
  }
  const key = (name: string, dir: boolean) => (prefix ? `${prefix}/${name}` : name) + (dir ? '/' : '');
  function toggle(k: string) {
    const next = new Set(picked);
    if (next.has(k)) next.delete(k);
    else next.add(k);
    picked = next;
  }

  async function chooseTarget() {
    const p = await open({ directory: true, multiple: false });
    if (typeof p === 'string') target = p;
  }
  async function restore() {
    busy = true;
    error = '';
    report = null;
    try {
      report = await backupIpc.restore(plan.id, version.name, picked.size ? [...picked] : null, target);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function verify() {
    checking = true;
    check = '';
    try {
      const r = await backupIpc.verify(plan.id, version.name);
      check = r.missing.length + r.wrong_size.length === 0 ? `All ${r.checked} files are there and the right size.` : `${r.missing.length} missing and ${r.wrong_size.length} with the wrong size, out of ${r.checked}.`;
    } catch (e) {
      check = String(e);
    } finally {
      checking = false;
    }
  }
</script>

<Dialog title="Restore from a backup" {onclose}>
  <div class="wrap">
    <p class="intro">Version of <strong>{when}</strong>: {version.files.toLocaleString()} files, {formatBytes(version.bytes)}. Nothing that is already in the target folder is replaced.</p>

    <nav class="crumbs" aria-label="Location">
      <button onclick={() => up(0)}>Whole backup</button>
      {#each path as part, i (i + part)}
        <span aria-hidden="true">›</span>
        <button translate="no" onclick={() => up(i + 1)}>{part}</button>
      {/each}
    </nav>

    <div class="list" role="group" aria-label="Files in this version">
      {#if tree}
        {#each tree.folders as f (f.name)}
          <div class="row">
            <input type="checkbox" aria-label={`Restore ${f.name}`} checked={picked.has(key(f.name, true))} onchange={() => toggle(key(f.name, true))} />
            <button class="name" translate="no" onclick={() => enter(f.name)}><Icon name="folder" size={15} /> {f.name}</button>
            <span class="meta tabular">{f.files.toLocaleString()} files · {formatBytes(f.bytes)}</span>
          </div>
        {/each}
        {#each tree.files as f (f.name)}
          <div class="row">
            <input type="checkbox" aria-label={`Restore ${f.name}`} checked={picked.has(key(f.name, false))} onchange={() => toggle(key(f.name, false))} />
            <span class="name plain" translate="no"><Icon name="file" size={15} /> {f.name}</span>
            <span class="meta tabular">{formatBytes(f.bytes)}</span>
          </div>
        {/each}
        {#if tree.folders.length + tree.files.length === 0}<p class="empty">Nothing in this folder.</p>{/if}
      {:else}
        <p class="empty">Loading…</p>
      {/if}
    </div>
    <p class="hint">{picked.size === 0 ? 'Nothing ticked: everything in this version will be restored.' : `${picked.size} ticked.`}</p>

    <div class="pick">
      <span class="path" translate="no" title={target}>{target || 'No folder chosen'}</span>
      <button class="plain" onclick={chooseTarget}>Choose folder to restore into</button>
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}
    {#if check}<p class="ok" role="status">{check}</p>{/if}
    {#if report}
      <p class="ok" role="status">
        Restored {report.restored.toLocaleString()} files ({formatBytes(report.bytes)}){report.renamed ? `, ${report.renamed} under a new name because the old one was taken` : ''}{report.failed.length ? `. ${report.failed.length} couldn't be restored.` : '.'}
      </p>
    {/if}
  </div>
  {#snippet actions()}
    <button class="plain" onclick={verify} disabled={checking}>{checking ? 'Checking…' : 'Check this version'}</button>
    <button class="plain" onclick={onclose}>Close</button>
    <button class="primary" onclick={restore} disabled={busy || !target}>{busy ? 'Restoring…' : 'Restore'}</button>
  {/snippet}
</Dialog>

<style>
  .wrap { display: grid; gap: 10px; width: min(680px, 84vw); }
  .intro { margin: 0; color: var(--text-2); }
  .crumbs { display: flex; flex-wrap: wrap; gap: 4px; align-items: center; color: var(--text-3); }
  .crumbs button { border: 0; background: none; color: var(--accent-text); cursor: pointer; padding: 2px 4px; }
  .list { max-height: 280px; overflow: auto; border: 1px solid var(--border-strong); border-radius: var(--radius-s); background: var(--surface-1); }
  .row { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 10px; align-items: center; padding: 6px 12px; border-bottom: 1px solid var(--border); }
  .row:last-child { border-bottom: 0; }
  .name { display: inline-flex; align-items: center; gap: 7px; border: 0; background: none; color: inherit; text-align: left; cursor: pointer; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 0; }
  .name.plain { cursor: default; }
  .meta { color: var(--text-3); font-size: 12.5px; }
  .empty { padding: 14px; color: var(--text-3); margin: 0; }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 0; }
  .pick { display: flex; gap: 10px; align-items: center; }
  .path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 8px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px dashed var(--border-strong); color: var(--text-2); }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.55; cursor: default; }
  .error { color: var(--danger); margin: 0; }
  .ok { color: var(--ok); margin: 0; }
</style>
