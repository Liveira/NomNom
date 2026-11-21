<script lang="ts">
  import { open, save } from '@tauri-apps/plugin-dialog';
  import { listen } from '@tauri-apps/api/event';
  import { onDestroy } from 'svelte';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import { archiveIpc, launcherIpc, type ZipListing } from '../../lib/ipc';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';

  const TABS = [
    { id: 'make', label: 'Make a ZIP' },
    { id: 'open', label: 'Open a ZIP' },
  ] as const;
  let tab = $state<(typeof TABS)[number]['id']>('make');
  let sources = $state<string[]>([]);
  let zip = $state('');
  let listing = $state<ZipListing | null>(null);
  let busy = $state(false);
  let progress = $state<{ done: number; total: number } | null>(null);
  let error = $state('');
  let done = $state<{ text: string; path: string } | null>(null);

  let stop: (() => void) | null = null;
  void listen<{ done: number; total: number }>('archive://progress', (e) => (progress = e.payload)).then((u) => (stop = u));
  onDestroy(() => stop?.());

  const leaf = (p: string) => p.split(/[\\/]+/).filter(Boolean).pop() ?? p;
  const parent = (p: string) => p.replace(/[\\/]+[^\\/]*[\\/]*$/, '');

  async function addFiles() {
    const p = await open({ multiple: true });
    if (p) sources = [...new Set([...sources, ...(Array.isArray(p) ? p : [p])])];
  }
  async function addFolder() {
    const p = await open({ directory: true, multiple: true });
    if (p) sources = [...new Set([...sources, ...(Array.isArray(p) ? p : [p])])];
  }

  async function make() {
    error = '';
    done = null;
    const base = sources.length === 1 ? leaf(sources[0]).replace(/\.[^.]*$/, '') : 'archive';
    const dest = await save({ defaultPath: `${base}.zip`, filters: [{ name: 'ZIP file', extensions: ['zip'] }] });
    if (!dest) return;
    busy = true;
    progress = null;
    try {
      const r = await archiveIpc.create(sources, dest);
      done = { text: `${r.files === 1 ? '1 file' : `${r.files} files`} packed (${formatBytes(r.bytes)}).`, path: r.path };
      sources = [];
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      progress = null;
    }
  }

  async function choose() {
    error = '';
    done = null;
    const p = await open({ multiple: false, filters: [{ name: 'ZIP file', extensions: ['zip'] }] });
    if (typeof p !== 'string') return;
    zip = p;
    listing = null;
    try {
      listing = await archiveIpc.list(p);
    } catch (e) {
      error = String(e);
    }
  }

  async function unpack() {
    error = '';
    done = null;
    const into = await open({ directory: true, multiple: false, defaultPath: parent(zip) });
    if (typeof into !== 'string') return;
    busy = true;
    progress = null;
    try {
      const r = await archiveIpc.extract(zip, into);
      done = { text: `${r.files === 1 ? '1 file' : `${r.files} files`} unpacked (${formatBytes(r.bytes)}).`, path: r.folder };
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
      progress = null;
    }
  }
  const fraction = $derived(progress && progress.total > 0 ? Math.min(1, progress.done / progress.total) : null);
</script>

<header class="head">
  <h1><Icon name="package" size={24} /> ZIP files</h1>
  <p class="sub">Pack files and folders into a ZIP, or unpack one. A ZIP is always unpacked into a new folder, nothing is replaced, and one that tries to write outside it, or that unpacks to far more than it weighs, is refused. Files with a password, 7z and RAR are not supported yet.</p>
</header>

<div class="tabs" role="tablist" aria-label="What to do">
  {#each TABS as t (t.id)}
    <button role="tab" aria-selected={tab === t.id} class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
  {/each}
</div>

<section class="pane" role="tabpanel">
  {#if tab === 'make'}
    <div class="row">
      <button class="plain" disabled={busy} onclick={() => void addFiles()}>Add files…</button>
      <button class="plain" disabled={busy} onclick={() => void addFolder()}>Add a folder…</button>
      {#if sources.length > 0}<button class="plain" disabled={busy} onclick={() => (sources = [])}>Clear</button>{/if}
    </div>
    {#if sources.length === 0}
      <p class="hint">Add what should go in the ZIP.</p>
    {:else}
      <ul class="list">
        {#each sources as s (s)}
          <li><span translate="no" title={s}>{leaf(s)}</span><span class="hint" translate="no">{parent(s)}</span><button class="plain small" aria-label={`Remove ${leaf(s)}`} onclick={() => (sources = sources.filter((x) => x !== s))}><Icon name="x" size={13} /></button></li>
        {/each}
      </ul>
      <div class="row"><button class="primary" disabled={busy} onclick={() => void make()}>{busy ? 'Packing…' : 'Make the ZIP…'}</button></div>
    {/if}
  {:else}
    <div class="row"><button class="plain" disabled={busy} onclick={() => void choose()}>Choose a ZIP…</button>{#if zip}<span class="hint" translate="no">{leaf(zip)}</span>{/if}</div>
    {#if listing}
      <p class="hint tabular">{listing.count} {listing.count === 1 ? 'item' : 'items'}, {formatBytes(listing.unpacked)} once unpacked (the file is {formatBytes(listing.packed)}).</p>
      {#if listing.encrypted}<p class="bad" role="alert">This ZIP has a password, which is not supported yet.</p>{/if}
      <ul class="list files">
        {#each listing.items as it (it.name)}
          <li><span translate="no">{it.name}</span><span class="hint tabular">{it.is_dir ? '' : formatBytes(it.size)}</span></li>
        {/each}
      </ul>
      {#if listing.count > listing.items.length}<p class="hint">Showing the first {listing.items.length}.</p>{/if}
      <div class="row"><button class="primary" disabled={busy || listing.encrypted} onclick={() => void unpack()}>{busy ? 'Unpacking…' : 'Unpack into a new folder…'}</button></div>
    {/if}
  {/if}

  {#if busy}
    <div class="prog">
      <ProgressBar value={fraction} label="Progress" />
      <button class="plain small" onclick={() => void archiveIpc.cancel()}>Stop</button>
    </div>
  {/if}
  {#if done}
    <p class="ok" role="status">{done.text}
      <button class="plain small" onclick={() => void launcherIpc.openPath(done!.path, true)}>Show it</button>
    </p>
  {/if}
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
</section>

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; margin: 0; }
  .ok { color: var(--ok); font-size: 13px; margin: 0; display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .tabs { display: flex; gap: 4px; padding: 3px; border-radius: 18px; background: var(--surface-2); border: 1px solid var(--border); width: fit-content; margin-bottom: 16px; }
  .tabs button { border: 0; background: none; color: var(--text-2); padding: 6px 16px; border-radius: 99px; cursor: pointer; font: inherit; font-weight: 600; font-size: 13px; }
  .tabs button.on { background: var(--surface-3); color: var(--text); }
  .pane { display: grid; gap: 12px; max-width: 760px; }
  .row { display: flex; gap: 8px; align-items: center; flex-wrap: wrap; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; }
  .list li { display: flex; justify-content: space-between; gap: 12px; align-items: center; padding: 6px 10px; border-radius: var(--radius-xs); background: var(--surface-1); border: 1px solid var(--border); font-size: 13px; }
  .list li span:first-child { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .files { max-height: 320px; overflow: auto; }
  .prog { display: flex; gap: 12px; align-items: center; }
  .prog :global(.progress), .prog :global([role='progressbar']) { flex: 1; }
  button.plain, button.primary { display: inline-flex; align-items: center; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover:not(:disabled) { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.small { padding: 3px 10px; font-size: 12.5px; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
