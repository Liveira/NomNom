<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { ftIpc, pdfIpc, type PdfInfo } from '../../lib/ipc';

  let mode = $state<'merge' | 'split'>('merge');
  let files = $state<PdfInfo[]>([]);
  let splitHow = $state<'each' | 'pages'>('each');
  let pages = $state('');
  let busy = $state(false);
  let error = $state('');
  let made = $state<string[]>([]);

  const ok = $derived(files.every((f) => !f.problem));
  const canRun = $derived(!busy && files.length > 0 && ok && (mode === 'merge' ? files.length >= 2 : splitHow === 'each' || pages.trim() !== ''));

  async function choose() {
    error = '';
    made = [];
    const picked = await open({ multiple: mode === 'merge', filters: [{ name: 'PDF', extensions: ['pdf'] }] });
    if (!picked) return;
    const list = Array.isArray(picked) ? picked : [picked];
    const info = await pdfIpc.info(list);
    files = mode === 'merge' ? [...files, ...info.filter((i) => !files.some((f) => f.path === i.path))] : info.slice(0, 1);
  }

  function move(i: number, d: number) {
    const j = i + d;
    if (j < 0 || j >= files.length) return;
    const next = [...files];
    [next[i], next[j]] = [next[j], next[i]];
    files = next;
  }

  function setMode(m: 'merge' | 'split') {
    mode = m;
    files = [];
    made = [];
    error = '';
  }

  async function run() {
    busy = true;
    error = '';
    made = [];
    try {
      made = mode === 'merge' ? [await pdfIpc.merge(files.map((f) => f.path))] : await pdfIpc.split(files[0].path, splitHow, pages);
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<section class="card pdf" aria-labelledby="pdf-h">
  <div class="head">
    <h2 id="pdf-h"><Icon name="file-text" size={18} /> PDF tools</h2>
    <div class="seg" role="radiogroup" aria-label="PDF tool">
      <button role="radio" aria-checked={mode === 'merge'} class:on={mode === 'merge'} onclick={() => setMode('merge')}>Join PDFs</button>
      <button role="radio" aria-checked={mode === 'split'} class:on={mode === 'split'} onclick={() => setMode('split')}>Take pages out</button>
    </div>
  </div>
  <p class="hint">
    {mode === 'merge' ? 'Put several PDFs into one, in the order shown.' : 'Save every page as its own file, or only the pages you choose.'}
    The original files are never changed.
  </p>

  <div class="row">
    <button class="plain" onclick={choose} disabled={busy}><Icon name="plus" size={15} /> {mode === 'merge' ? 'Add PDFs' : 'Choose a PDF'}</button>
    {#if files.length > 0}<button class="plain" onclick={() => (files = [])} disabled={busy}>Clear list</button>{/if}
  </div>

  {#if files.length > 0}
    <ol class="list">
      {#each files as f, i (f.path)}
        <li class:bad={!!f.problem}>
          <span class="n tabular">{i + 1}</span>
          <span class="name" translate="no" title={f.path}>{f.name}</span>
          <span class="pg tabular">{f.problem ?? `${f.pages} pages`}</span>
          {#if mode === 'merge'}
            <button class="mini" aria-label="Move up" onclick={() => move(i, -1)} disabled={i === 0 || busy}><Icon name="arrow-up" size={14} /></button>
            <button class="mini" aria-label="Move down" onclick={() => move(i, 1)} disabled={i === files.length - 1 || busy}><Icon name="arrow-down" size={14} /></button>
            <button class="mini" aria-label="Remove" onclick={() => (files = files.filter((x) => x.path !== f.path))} disabled={busy}><Icon name="x" size={14} /></button>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}

  {#if mode === 'split' && files.length > 0}
    <div class="how" role="radiogroup" aria-label="Which pages">
      <label><input type="radio" bind:group={splitHow} value="each" /> Every page as its own file</label>
      <label><input type="radio" bind:group={splitHow} value="pages" /> Only these pages</label>
      {#if splitHow === 'pages'}
        <input class="spec" bind:value={pages} placeholder="For example 1-3, 5, 8-" aria-label="Pages to keep" />
      {/if}
    </div>
  {/if}

  <div class="row">
    <button class="primary" onclick={run} disabled={!canRun}>{busy ? 'Working…' : mode === 'merge' ? 'Join into one PDF' : 'Take the pages out'}</button>
  </div>

  {#if error}<p class="error" role="alert">{error}</p>{/if}
  {#if made.length > 0}
    <div class="done" role="status">
      <strong>{made.length === 1 ? 'Saved 1 file' : `Saved ${made.length} files`}</strong>
      <ul>
        {#each made.slice(0, 4) as m (m)}
          <li><span translate="no" title={m}>{m.split(/[\\/]/).pop()}</span> <button class="link" onclick={() => ftIpc.open(m, true)}>Show in folder</button></li>
        {/each}
      </ul>
    </div>
  {/if}
</section>

<style>
  .pdf { max-width: 760px; margin: 16px 0 14px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 14px 16px; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 12px; flex-wrap: wrap; }
  h2 { display: flex; align-items: center; gap: 8px; font-size: 16px; }
  .seg { display: inline-flex; padding: 3px; border-radius: var(--radius-pill); background: var(--surface-2); border: 1px solid var(--border); }
  .seg button { border: 0; background: transparent; color: var(--text-2); padding: 5px 14px; border-radius: var(--radius-pill); cursor: pointer; }
  .seg button.on { background: var(--surface-3); color: var(--text); font-weight: 700; }
  .hint { color: var(--text-2); margin: 8px 0 12px; font-size: 13px; }
  .row { display: flex; gap: 10px; margin: 10px 0; flex-wrap: wrap; }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button:disabled { opacity: 0.5; cursor: default; }
  .list { list-style: none; margin: 8px 0; padding: 0; display: grid; gap: 6px; }
  .list li { display: grid; grid-template-columns: 26px minmax(0, 1fr) auto auto auto auto; gap: 8px; align-items: center; padding: 8px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .list li.bad { border-color: var(--danger); }
  .n { color: var(--text-3); }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pg { color: var(--text-3); font-size: 12.5px; }
  li.bad .pg { color: var(--danger); }
  .mini { display: grid; place-items: center; width: 28px; height: 28px; border: 1px solid var(--border); border-radius: 8px; background: var(--surface-3); color: inherit; cursor: pointer; }
  .how { display: flex; flex-wrap: wrap; gap: 8px 18px; align-items: center; margin: 8px 0; color: var(--text-2); }
  .how label { display: inline-flex; gap: 6px; align-items: center; cursor: pointer; }
  .spec { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); min-width: 220px; user-select: text; }
  .error { color: var(--danger); margin: 8px 0 0; }
  .done { margin-top: 10px; padding: 10px 14px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--ok); }
  .done ul { list-style: none; margin: 6px 0 0; padding: 0; display: grid; gap: 4px; }
  .link { border: 0; background: none; color: var(--accent-text); text-decoration: underline; cursor: pointer; padding: 0; margin-left: 8px; }
</style>
