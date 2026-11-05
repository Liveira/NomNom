<script lang="ts">
  import { tick } from 'svelte';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import { app } from '../../app/state.svelte';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { formatBytes } from '../../lib/format';
  import {
    ftIpc,
    onFileToolsEvent,
    onFileToolsInbox,
    type FtConversion,
    type FtFile,
    type FtOptions,
    type FtOutcome,
    type FtPreset,
    type FtTarget,
  } from '../../lib/ipc';
  import {
    controls,
    fileName,
    groupTargets,
    KIND_NAMES,
    parseCount,
    parseTime,
    presetsFor,
    savedLine,
    summaryLine,
    warningFor,
  } from './logic';

  const defaults = (): FtOptions => ({
    image_quality: 85,
    video_quality: 'balanced',
    max_width: null,
    max_height: null,
    keep_metadata: true,
    background: [255, 255, 255],
    fps: null,
    start_secs: null,
    end_secs: null,
    audio_kbps: null,
    pdf_page: 'fit',
    combine: false,
  });

  let files = $state<FtFile[]>([]);
  let convs = $state<FtConversion[]>([]);
  let presets = $state<FtPreset[]>([]);
  let allConvs = $state<FtConversion[]>([]);
  let target = $state<FtTarget | ''>('');
  let options = $state<FtOptions>(defaults());
  let phase = $state<'idle' | 'running' | 'done'>('idle');
  let progress = $state<Record<number, number>>({});
  let current = $state(-1);
  let runFiles = $state<string[]>([]);
  let outcomes = $state<FtOutcome[]>([]);
  let ranTarget = $state('');
  let error = $state('');
  let dragging = $state(false);
  let openError = $state('');
  let resultsEl = $state<HTMLElement | null>(null);

  // Typed fields are checked when the user starts, so half-typed numbers never reach the app.
  let widthText = $state('');
  let heightText = $state('');
  let fpsText = $state('');
  let startText = $state('');
  let endText = $state('');
  let kbpsText = $state('');
  let colour = $state('#ffffff');

  let outDir = $state<string | null>(null);
  try {
    outDir = localStorage.getItem('nomnom.filetools.out');
  } catch {
    /* storage can be unavailable */
  }
  function rememberOut() {
    try {
      if (outDir) localStorage.setItem('nomnom.filetools.out', outDir);
      else localStorage.removeItem('nomnom.filetools.out');
    } catch {
      /* storage can be unavailable */
    }
  }

  const usable = $derived(files.filter((f) => !f.problem));
  const kinds = $derived([...new Set(usable.map((f) => f.kind!))]);
  const groups = $derived(groupTargets(convs));
  const chosen = $derived(convs.find((c) => c.target === target) ?? null);
  const ctl = $derived(target ? controls(target, kinds, usable.length) : null);
  const warning = $derived(target ? warningFor(usable, target) : null);
  const fitPresets = $derived(presetsFor(presets, convs, kinds));
  const needsFfmpeg = $derived(
    kinds.some((k) => k === 'video' || k === 'audio') && allConvs.some((c) => c.needs === 'FFmpeg'),
  );
  const busy = $derived(phase === 'running');

  async function loadInfo() {
    const info = await ftIpc.info();
    presets = info.presets;
    allConvs = info.conversions;
  }

  async function refresh() {
    if (files.length === 0) {
      convs = [];
      target = '';
      return;
    }
    const r = await ftIpc.inspect(files.map((f) => f.path));
    files = r.files;
    convs = r.conversions;
    if (target && !convs.some((c) => c.target === target && c.available)) target = '';
  }

  async function addPaths(paths: string[]) {
    if (busy || paths.length === 0) return;
    error = '';
    const have = new Set(files.map((f) => f.path.toLowerCase()));
    const fresh = paths.filter((p) => !have.has(p.toLowerCase()));
    const merged = [...files.map((f) => f.path), ...fresh];
    try {
      const r = await ftIpc.inspect(merged);
      files = r.files;
      convs = r.conversions;
      if (target && !convs.some((c) => c.target === target && c.available)) target = '';
      if (phase === 'done') {
        phase = 'idle';
        outcomes = [];
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function chooseFiles() {
    const picked = await open({ multiple: true, directory: false });
    if (Array.isArray(picked)) await addPaths(picked);
    else if (typeof picked === 'string') await addPaths([picked]);
  }

  async function removeFile(path: string) {
    if (busy) return;
    files = files.filter((f) => f.path !== path);
    await refresh();
  }

  function clearAll() {
    if (busy) return;
    files = [];
    convs = [];
    target = '';
    outcomes = [];
    phase = 'idle';
    error = '';
  }

  async function chooseOut() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === 'string') {
      outDir = picked;
      rememberOut();
    }
  }

  function applyPreset(id: string) {
    const p = presets.find((x) => x.id === id);
    if (!p) return;
    target = p.target;
    options = { ...p.options };
    widthText = p.options.max_width?.toString() ?? '';
    heightText = p.options.max_height?.toString() ?? '';
    fpsText = p.options.fps?.toString() ?? '';
    kbpsText = p.options.audio_kbps?.toString() ?? '';
    startText = '';
    endText = '';
  }

  function hexToRgb(hex: string): [number, number, number] {
    const m = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i.exec(hex);
    return m ? [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)] : [255, 255, 255];
  }

  function gather(): FtOptions | string {
    const w = parseCount(widthText, 1, 16384);
    const h = parseCount(heightText, 1, 16384);
    const f = parseCount(fpsText, 1, 120);
    const k = parseCount(kbpsText, 32, 512);
    const s = parseTime(startText);
    const e = parseTime(endText);
    if (w === 'bad' || h === 'bad') return 'Width and height must be whole numbers from 1 to 16384.';
    if (f === 'bad') return 'Frames per second must be a whole number from 1 to 120.';
    if (k === 'bad') return 'Bitrate must be a whole number from 32 to 512.';
    if (s === 'bad' || e === 'bad') return 'Times look like 90 or 1:30.';
    if (s != null && e != null && e <= s) return 'The end time must be after the start time.';
    return {
      ...options,
      max_width: ctl?.size ? w : null,
      max_height: ctl?.size ? h : null,
      fps: ctl?.fps ? f : null,
      audio_kbps: ctl?.audioKbps ? k : null,
      start_secs: ctl?.trim ? s : null,
      end_secs: ctl?.trim ? e : null,
      background: hexToRgb(colour),
      combine: ctl?.combine ? options.combine : false,
    };
  }

  async function start() {
    if (!target || usable.length === 0) return;
    const opts = gather();
    if (typeof opts === 'string') {
      error = opts;
      return;
    }
    error = '';
    openError = '';
    outcomes = [];
    progress = {};
    current = -1;
    runFiles = usable.map((f) => f.path);
    ranTarget = chosen?.label ?? String(target);
    phase = 'running';
    // The results sit below the options; bring them into view.
    void tick().then(() => resultsEl?.scrollIntoView({ block: 'nearest' }));
    try {
      outcomes = await ftIpc.run({ paths: runFiles, target, options: opts, out_dir: outDir });
      phase = 'done';
    } catch (e) {
      error = String(e);
      phase = 'idle';
    }
  }

  async function openResult(path: string, reveal: boolean) {
    openError = '';
    try {
      await ftIpc.open(path, reveal);
    } catch (e) {
      openError = String(e);
    }
  }

  const sizeOf = (n: number) => formatBytes(n);

  $effect(() => {
    void loadInfo();
    void ftIpc.takeInbox().then((p) => addPaths(p));
    const unlisteners: Promise<() => void>[] = [
      onFileToolsEvent((e) => {
        if (e.type === 'started') {
          current = e.index;
          progress = { ...progress, [e.index]: 0 };
        } else if (e.type === 'progress') {
          progress = { ...progress, [e.index]: e.fraction };
        } else if (e.type === 'finished') {
          progress = { ...progress, [e.index]: 1 };
          const next = [...outcomes];
          next[e.index] = e.outcome;
          outcomes = next;
        }
      }),
      onFileToolsInbox(() => void ftIpc.takeInbox().then((p) => addPaths(p))),
      getCurrentWebview().onDragDropEvent((ev) => {
        const t = ev.payload.type;
        if (t === 'enter' || t === 'over') dragging = true;
        else if (t === 'leave') dragging = false;
        else if (t === 'drop') {
          dragging = false;
          void addPaths(ev.payload.paths);
        }
      }),
    ];
    return () => unlisteners.forEach((u) => void u.then((f) => f()));
  });
</script>

<h1>File Tools</h1>

<section class="card note">
  <p>Convert pictures, sound, video and documents. Your original files are never changed, and nothing is replaced.</p>
</section>

<section class="card drop" class:over={dragging} aria-label="Files to convert">
  {#if files.length === 0}
    <div class="empty">
      <CatNap width={96} />
      <div>
        <p class="big">Drop files here</p>
        <p class="hint">Pictures, sound, video, text, Markdown and Office files.</p>
      </div>
    </div>
  {:else}
    <ul class="files">
      {#each files as f (f.path)}
        <li class:bad={!!f.problem}>
          <span class="name" title={f.path}>{f.name}</span>
          <span class="meta">
            {#if f.problem}{f.problem}{:else}{f.kind ? KIND_NAMES[f.kind] : ''} · {sizeOf(f.size)}{/if}
          </span>
          <button class="x" onclick={() => removeFile(f.path)} disabled={busy} aria-label={`Remove ${f.name}`}>×</button>
        </li>
      {/each}
    </ul>
  {/if}
  <div class="row end">
    {#if files.length > 0}<button class="plain" onclick={clearAll} disabled={busy}>Clear list</button>{/if}
    <button class="plain" onclick={chooseFiles} disabled={busy}>Choose files</button>
  </div>
</section>

{#if needsFfmpeg}
  <section class="card tip">
    <p>Video and sound need FFmpeg. It's a free tool NomNom can download for you.</p>
    <button class="plain" onclick={() => app.go('downloads')}>Install FFmpeg in NomNom Downloader</button>
  </section>
{/if}

{#if usable.length > 0}
  <section class="card">
    <div class="row">
      <label class="label" for="target">Convert to</label>
      <select id="target" bind:value={target} disabled={busy}>
        <option value="" disabled>Pick a format</option>
        {#each groups as g (g.title)}
          <optgroup label={g.title}>
            {#each g.items as c (c.target + c.kind)}
              <option value={c.target} disabled={!c.available}>{c.label}{c.available ? '' : ` (needs ${c.needs})`}</option>
            {/each}
          </optgroup>
        {/each}
      </select>
    </div>
    {#if convs.length === 0}
      <p class="hint">These files are different kinds, so there's no one format for all of them. Convert them in separate batches.</p>
    {/if}

    {#if fitPresets.length > 0}
      <div class="row">
        <label class="label" for="preset">Or start from a preset</label>
        <select id="preset" disabled={busy} onchange={(e) => { applyPreset(e.currentTarget.value); e.currentTarget.value = ''; }}>
          <option value="">Choose…</option>
          {#each fitPresets as p (p.id)}<option value={p.id}>{p.name}: {p.description}</option>{/each}
        </select>
      </div>
    {/if}

    {#if warning}<p class="warn" role="note">{warning}</p>{/if}

    {#if ctl}
      <div class="opts">
        {#if ctl.quality}
          <label class="opt">Quality
            <span class="inline"><input type="range" min="1" max="100" bind:value={options.image_quality} disabled={busy} />
            <output>{options.image_quality}{options.image_quality === 100 && target === 'webp' ? ' (lossless)' : ''}</output></span>
          </label>
        {/if}
        {#if ctl.videoQuality}
          <label class="opt">Quality
            <select bind:value={options.video_quality} disabled={busy}>
              <option value="small">Smaller file</option>
              <option value="balanced">Balanced</option>
              <option value="high">Best looking</option>
            </select>
          </label>
        {/if}
        {#if ctl.size}
          <div class="opt">Largest size
            <span class="inline">
              <input class="num" inputmode="numeric" placeholder="width" aria-label="Largest width in pixels" bind:value={widthText} disabled={busy} />
              ×
              <input class="num" inputmode="numeric" placeholder="height" aria-label="Largest height in pixels" bind:value={heightText} disabled={busy} />
              <span class="hint">pixels. Never makes things bigger.</span>
            </span>
          </div>
        {/if}
        {#if ctl.fps}
          <label class="opt">Frames per second
            <input class="num" inputmode="numeric" placeholder={target === 'gif' ? '12' : 'as is'} bind:value={fpsText} disabled={busy} />
          </label>
        {/if}
        {#if ctl.trim}
          <div class="opt">Only part of it
            <span class="inline">
              <input class="num" placeholder="from 0:00" aria-label="Start time" bind:value={startText} disabled={busy} />
              to
              <input class="num" placeholder="end" aria-label="End time" bind:value={endText} disabled={busy} />
            </span>
          </div>
        {/if}
        {#if ctl.audioKbps}
          <label class="opt">Bitrate
            <span class="inline"><input class="num" inputmode="numeric" placeholder="192" bind:value={kbpsText} disabled={busy} /> <span class="hint">kbps</span></span>
          </label>
        {/if}
        {#if ctl.metadata}
          <label class="check"><input type="checkbox" bind:checked={options.keep_metadata} disabled={busy} />
            <span>Keep camera details and colour profile</span></label>
        {/if}
        {#if ctl.background}
          <label class="opt">Fill transparent areas with
            <input type="color" bind:value={colour} disabled={busy} aria-label="Fill colour" />
          </label>
        {/if}
        {#if ctl.pdfPage}
          <label class="opt">Page size
            <select bind:value={options.pdf_page} disabled={busy}>
              <option value="fit">Same as the picture</option>
              <option value="a4">A4</option>
            </select>
          </label>
        {/if}
        {#if ctl.combine}
          <label class="check"><input type="checkbox" bind:checked={options.combine} disabled={busy} />
            <span>Put all pictures in one PDF</span></label>
        {/if}
      </div>
    {/if}

    <div class="row">
      <div>
        <div class="label">Save results</div>
        <div class="hint path">{outDir ?? 'Next to each original file'}</div>
      </div>
      <div class="buttons">
        <button class="plain" onclick={chooseOut} disabled={busy}>Choose folder</button>
        {#if outDir}<button class="plain" onclick={() => { outDir = null; rememberOut(); }} disabled={busy}>Next to originals</button>{/if}
      </div>
    </div>

    {#if error}<p class="error" role="alert">{error}</p>{/if}

    <div class="row end">
      {#if busy}
        <button class="plain" onclick={() => ftIpc.cancel()}>Stop</button>
      {/if}
      <button class="primary" onclick={start} disabled={busy || !target || !chosen?.available}>
        {busy ? 'Converting…' : `Convert ${usable.length === 1 ? '1 file' : `${usable.length} files`}`}
      </button>
    </div>
  </section>
{/if}

{#if phase !== 'idle'}
  <section class="card" aria-live="polite" bind:this={resultsEl}>
    <h2>{phase === 'running' ? `Converting to ${ranTarget}` : summaryLine(outcomes, ranTarget)}</h2>
    <ul class="results">
      {#each runFiles as path, i (path)}
        {@const o = outcomes[i]}
        <li>
          <div class="line">
            <span class="name" title={path}>{fileName(path)}</span>
            {#if o}
              <span class="status" class:ok={o.status === 'done'} class:err={o.status === 'failed'}>
                {o.status === 'done' ? 'Done' : o.status === 'failed' ? 'Failed' : 'Stopped'}
              </span>
            {:else if current === i}
              <span class="status">{Math.round((progress[i] ?? 0) * 100)}%</span>
            {:else}
              <span class="status">Waiting</span>
            {/if}
          </div>
          {#if !o && current === i}<ProgressBar value={progress[i] ?? null} label={`Converting ${fileName(path)}`} />{/if}
          {#if o}
            {#if o.status === 'done' && o.dst}
              <div class="detail">
                <span class="path" title={o.dst}>{fileName(o.dst)}</span>
                <span class="hint">{savedLine(o.in_bytes, o.out_bytes)}</span>
                <span class="buttons">
                  <button class="link" onclick={() => openResult(o.dst!, false)}>Open</button>
                  <button class="link" onclick={() => openResult(o.dst!, true)}>Show in folder</button>
                </span>
              </div>
            {/if}
            {#if o.message}<p class="hint err">{o.message}</p>{/if}
            {#each o.notes as n (n)}<p class="hint">{n}</p>{/each}
          {/if}
        </li>
      {/each}
    </ul>
    {#if openError}<p class="error" role="alert">{openError}</p>{/if}
  </section>
{/if}

<style>
  h1 { margin-bottom: 18px; }
  h2 { font-size: 16px; margin: 0 0 8px; }
  .card { max-width: 760px; margin-bottom: 14px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 14px 16px; }
  .note p, .tip p { margin: 0; color: var(--text-2); }
  .tip { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .drop { border-style: dashed; border-color: var(--border-strong); transition: background var(--dur-2) var(--ease-out), border-color var(--dur-2) var(--ease-out); }
  .drop.over { background: var(--surface-2); border-color: var(--accent); }
  .empty { display: flex; align-items: center; gap: 14px; padding: 8px 0; }
  .empty p { margin: 0; }
  .big { font-size: 16px; font-weight: 500; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 18px; }
  .row + .row { margin-top: 12px; }
  .row.end { justify-content: flex-end; margin-top: 12px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin: 2px 0 0; }
  .hint.err, .err { color: var(--danger); }
  .path { font-family: Consolas, monospace; word-break: break-all; user-select: text; }
  .buttons { display: flex; gap: 8px; flex: none; }
  .warn { margin: 12px 0 0; padding: 8px 10px; border-radius: var(--radius-s); background: var(--surface-2); color: var(--text-2); }
  .error { color: var(--danger); margin: 10px 0 0; }
  button, select, input:not([type='checkbox']):not([type='range']):not([type='color']) { padding: 7px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; }
  button { cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled, select:disabled, input:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.link { border: 0; background: none; padding: 0 4px; color: var(--accent-text); text-decoration: underline; }
  button.x { border: 0; background: none; padding: 0 8px; font-size: 18px; color: var(--text-3); }
  .files, .results { list-style: none; margin: 0 0 8px; padding: 0; max-height: 300px; overflow: auto; display: grid; gap: 2px; }
  .files li { display: flex; align-items: center; gap: 12px; padding: 3px 4px; border-radius: 6px; }
  .files li:hover { background: var(--surface-2); }
  .files li.bad .meta { color: var(--danger); }
  .name { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; flex: 1; min-width: 0; }
  .meta { color: var(--text-3); font-size: 12.5px; flex: none; }
  .opts + .row { margin-top: 16px; }
  .opts { display: grid; gap: 10px; margin-top: 14px; padding-top: 12px; border-top: 1px solid var(--border); }
  .opt { display: flex; align-items: center; justify-content: space-between; gap: 14px; }
  .inline { display: inline-flex; align-items: center; gap: 8px; }
  .num { width: 96px; }
  .check { display: flex; gap: 10px; align-items: center; color: var(--text-2); }
  .results li { padding: 6px 4px; border-bottom: 1px solid var(--border); }
  .results li:last-child { border-bottom: 0; }
  .line { display: flex; justify-content: space-between; gap: 12px; }
  .status { color: var(--text-3); flex: none; }
  .status.ok { color: var(--ok); }
  .status.err { color: var(--danger); }
  .detail { display: flex; align-items: center; gap: 12px; flex-wrap: wrap; margin-top: 2px; }
  .detail .path { font-size: 12.5px; }
  input[type='range'] { accent-color: var(--accent); }
  input[type='color'] { width: 44px; height: 30px; padding: 2px; }
</style>
