<script lang="ts">
  import Dialog from '../../lib/ui/Dialog.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { formatBytes } from '../../lib/format';
  import { attributeLabels, formatDate } from './logic';
  import { storage } from './store.svelte';

  let typed = $state('');
  let readonly = $state(false);
  let newName = $state('');

  const dlg = $derived(storage.dialog);

  // Reset the fields whenever a different dialog opens.
  $effect(() => {
    if (dlg?.type === 'confirm') {
      typed = '';
      readonly = false;
    }
    if (dlg?.type === 'rename') newName = dlg.node.name;
  });

  function close() {
    storage.dialog = null;
  }

  const verbs = { recycle: 'Move to the Recycle Bin', delete: 'Delete permanently', copy: 'Copy', move: 'Move', rename: 'Rename' } as const;

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
  <Dialog title={permanent ? 'Delete permanently?' : p.kind === 'recycle' ? 'Move to the Recycle Bin?' : p.kind === 'copy' ? 'Copy these items?' : 'Move these items?'} onclose={close} locked={dlg.busy}>
    {#if permanent}
      <p class="danger" role="alert"><strong>These items will NOT go to the Recycle Bin.</strong> They can't be brought back by Windows.</p>
    {/if}
    <p>
      {p.item_count.toLocaleString()} {p.item_count === 1 ? 'item' : 'items'}, {p.total_files.toLocaleString()} {p.total_files === 1 ? 'file' : 'files'}, {formatBytes(p.total_size)}
      {#if p.dest}<br />To <span class="mono">{p.dest}</span>{/if}
    </p>
    <ul class="items" aria-label="Items">
      {#each p.items.slice(0, 15) as it (it.path)}
        <li class:warn={it.warn}><span class="mono" title={it.path}>{it.path}</span><span class="sz tabular">{formatBytes(it.size)}</span></li>
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
