<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import {
    ipc,
    type Analysis,
    type Choice,
    type Container,
    type Kind,
  } from '../../lib/ipc';
  import { app } from '../../app/state.svelte';
  import { formatBytes, formatDuration, heightLabel } from '../../lib/format';
  import { AUDIO_FORMATS, VIDEO_FORMATS, formatNote } from '../../lib/formats';
  import { downloads } from './store.svelte';
  import JobItem from './JobItem.svelte';
  import ToolBanner from './ToolBanner.svelte';

  let url = $state('');
  let phase = $state<'idle' | 'loading' | 'error'>('idle');
  let error = $state('');
  let analysis = $state<Analysis | null>(null);
  let notice = $state<{ text: string; again?: boolean; path?: string } | null>(null);
  let confirmingAll = $state(false);

  let kind = $state<Kind>('video');
  let container = $state<Container>('original');
  let height = $state<number | null>(null);
  let seq = 0;

  const containers = $derived(kind === 'video' ? VIDEO_FORMATS : AUDIO_FORMATS);
  const info = $derived(analysis?.info ?? null);
  const hasVideo = $derived((info?.heights.length ?? 0) > 0);
  const choice = $derived<Choice>({ kind, container, height: kind === 'video' ? height : null });
  const note = $derived(formatNote(kind, container));
  const bestAudio = $derived(
    info?.audio_formats.reduce<(typeof info.audio_formats)[number] | null>(
      (best, f) => ((f.bitrate_kbps ?? 0) > (best?.bitrate_kbps ?? -1) ? f : best),
      null,
    ) ?? null,
  );

  /** The format chosen in Settings for this kind, or Original. */
  function defaultContainer(k: Kind): Container {
    const d = app.settings?.downloads;
    return (k === 'video' ? d?.default_video : d?.default_audio) ?? 'original';
  }

  function setKind(k: Kind) {
    kind = k;
    container = defaultContainer(k);
    height = null;
  }

  async function lookUp() {
    const value = url.trim();
    notice = null;
    confirmingAll = false;
    if (!value) {
      analysis = null;
      phase = 'idle';
      return;
    }
    const mine = ++seq;
    phase = 'loading';
    try {
      const result = await ipc.analyze(value);
      if (mine !== seq) return;
      analysis = result;
      phase = 'idle';
      // Start from the default type in Settings; a source with no picture can only be audio.
      const wanted: Kind = app.settings?.downloads.default_kind ?? 'video';
      const audioOnly = result.info.heights.length === 0 && !result.info.entries;
      setKind(audioOnly ? 'audio' : wanted);
    } catch (e) {
      if (mine !== seq) return;
      analysis = null;
      error = String(e);
      phase = 'error';
    }
  }

  function onPaste() {
    // Wait for the pasted text to land in the field, then look it up.
    setTimeout(() => void lookUp(), 0);
  }

  function reset() {
    seq++;
    url = '';
    analysis = null;
    phase = 'idle';
    confirmingAll = false;
  }

  async function startDownload(again = false) {
    if (!analysis) return;
    try {
      const out = await ipc.enqueue(url.trim(), choice, analysis.info.title, again);
      if (out.outcome === 'queued') {
        notice = { text: 'Added to the queue.' };
        reset();
      } else if (out.outcome === 'already_active') {
        notice = { text: 'This one is already in the queue.' };
      } else {
        notice = { text: "You already downloaded this.", again: true };
      }
    } catch (e) {
      notice = { text: String(e) };
    }
    void downloads.refresh();
  }

  async function startAll() {
    if (!info?.entries) return;
    let added = 0;
    for (const entry of info.entries) {
      try {
        const out = await ipc.enqueue(entry.url, choice, entry.title ?? undefined);
        if (out.outcome === 'queued') added++;
      } catch {
        /* skip entries that can't be queued; the rest still go */
      }
    }
    notice = { text: `Added ${added} of ${info.entries.length} to the queue.` };
    reset();
    void downloads.refresh();
  }

  const sizeFor = (h: number) => formatBytes(info?.size_by_height[String(h)]);
  const needsYtdlp = $derived(downloads.tools != null && downloads.tools.ytdlp.path == null);
  const needsFfmpeg = $derived(downloads.tools != null && downloads.tools.ffmpeg.path == null);
</script>

<h1>NomNom Downloader</h1>

{#if needsYtdlp}
  <ToolBanner tool="ytdlp" text="Downloads need yt-dlp (about 18 MB)." />
{/if}

<form class="input" onsubmit={(e) => { e.preventDefault(); void lookUp(); }}>
  <input
    type="text"
    placeholder="Paste a link"
    aria-label="Link"
    spellcheck="false"
    autocomplete="off"
    bind:value={url}
    onpaste={onPaste}
  />
  <button class="primary" type="submit" disabled={phase === 'loading' || !url.trim()}>
    {phase === 'loading' ? 'Looking…' : 'Look up'}
  </button>
</form>

{#if phase === 'error'}
  <p class="error" role="alert">{error}</p>
{/if}

{#if notice}
  <p class="notice" role="status">
    {notice.text}
    {#if notice.again}
      <button class="link" onclick={() => startDownload(true)}>Download again</button>
    {/if}
  </p>
{/if}

{#if phase === 'loading'}
  <section class="card skeleton" aria-busy="true">
    <div class="thumb"></div>
    <div class="lines"><div></div><div></div></div>
  </section>
{/if}

{#if analysis && info}
  <section class="card media">
    {#if info.thumbnail}
      <img class="thumb" src={info.thumbnail} alt="" referrerpolicy="no-referrer" />
    {:else}
      <div class="thumb"></div>
    {/if}
    <div class="details">
      <div class="title" title={info.title}>{info.title}</div>
      <div class="meta">
        <span>{analysis.site}</span>
        {#if info.uploader}<span>{info.uploader}</span>{/if}
        {#if info.duration_secs}<span class="tabular">{formatDuration(info.duration_secs)}</span>{/if}
        {#if info.is_live}<span>Live</span>{/if}
      </div>

      {#if info.entries}
        <p class="hint">This link is a playlist with {info.entries.length} items.</p>
        {#if confirmingAll}
          <div class="row">
            <span>Queue all {info.entries.length}?</span>
            <button class="primary" onclick={startAll}>Yes, download all</button>
            <button onclick={() => (confirmingAll = false)}>Cancel</button>
          </div>
        {:else}
          <button class="primary" onclick={() => (confirmingAll = true)}>
            <Icon name="download" size={16} /> Download all {info.entries.length}
          </button>
        {/if}
      {/if}

      <div class="options">
        <div class="field">
          <span class="label" id="kind-label">Type</span>
          <div class="segmented" role="radiogroup" aria-labelledby="kind-label">
            <button role="radio" aria-checked={kind === 'video'} class:on={kind === 'video'} disabled={!hasVideo && !info.entries} onclick={() => setKind('video')}>Video</button>
            <button role="radio" aria-checked={kind === 'audio'} class:on={kind === 'audio'} onclick={() => setKind('audio')}>Audio</button>
          </div>
        </div>
        <label class="field">
          <span class="label">Format</span>
          <select bind:value={container}>
            {#each containers as c (c.value)}<option value={c.value}>{c.label}</option>{/each}
          </select>
        </label>
        {#if kind === 'video'}
          <label class="field">
            <span class="label">Quality</span>
            <select bind:value={height}>
              <option value={null}>Best available</option>
              {#each info.heights as h (h)}
                <option value={h}>{heightLabel(h)}{sizeFor(h) ? ` · ${sizeFor(h)}` : ''}</option>
              {/each}
            </select>
          </label>
        {:else}
          <div class="field">
            <span class="label">Quality</span>
            <span class="static">
              Best available{bestAudio?.bitrate_kbps ? ` · ${bestAudio.bitrate_kbps} kbps source` : ''}
            </span>
          </div>
        {/if}
      </div>

      <p class="hint">{note}</p>
      {#if needsFfmpeg && !needsYtdlp}
        <ToolBanner tool="ffmpeg" compact text="FFmpeg is needed to merge video with audio and to convert formats (about 150 MB)." />
      {/if}

      {#if !info.entries}
        <div class="row">
          <button class="primary" onclick={() => startDownload()} disabled={needsYtdlp}>
            <Icon name="download" size={16} /> Download
          </button>
          <button onclick={reset}>Clear</button>
        </div>
      {/if}
    </div>
  </section>
{/if}

<h2>Queue</h2>
{#if downloads.active.length === 0}
  <div class="nap">
    <CatNap width={96} />
    <p class="empty">Nothing downloading.</p>
  </div>
{:else}
  <ul class="list">
    {#each downloads.active as job (job.id)}<li><JobItem {job} /></li>{/each}
  </ul>
{/if}

<div class="history-head">
  <h2>History</h2>
  {#if downloads.history.length > 0}
    <button class="link" onclick={() => ipc.clearHistory().then(() => downloads.refresh())}>Clear history</button>
  {/if}
</div>
{#if downloads.history.length === 0}
  <p class="empty">Finished downloads show up here.</p>
{:else}
  <ul class="list">
    {#each downloads.history as job (job.id)}<li><JobItem {job} /></li>{/each}
  </ul>
{/if}

<style>
  h1 { margin-bottom: 18px; }
  h2 { margin: 28px 0 10px; font-size: 16px; }
  .input { display: flex; gap: 8px; max-width: 760px; }
  input[type='text'] {
    flex: 1; min-width: 0; padding: 10px 12px; border-radius: var(--radius-s);
    border: 1px solid var(--border-strong); background: var(--surface-1); user-select: text;
  }
  input[type='text']::placeholder { color: var(--text-3); }
  button, select {
    padding: 8px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); cursor: pointer;
    transition: background var(--dur-1), transform var(--dur-1);
  }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:active:not(:disabled) { transform: scale(0.97); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; display: inline-flex; align-items: center; gap: 6px; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.link { border: 0; background: none; padding: 0 4px; color: var(--accent-text); text-decoration: underline; }
  .error { color: var(--danger); max-width: 760px; }
  .notice { color: var(--ok); max-width: 760px; }
  .card { margin-top: 16px; max-width: 760px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 14px; display: flex; gap: 14px; }
  .thumb { flex: none; width: 168px; height: 94px; border-radius: var(--radius-s); background: var(--surface-3); object-fit: cover; }
  .skeleton .lines { flex: 1; display: grid; gap: 10px; align-content: start; }
  .skeleton .lines div { height: 14px; border-radius: 4px; background: var(--surface-3); }
  .skeleton .lines div:last-child { width: 55%; }
  .details { flex: 1; min-width: 0; display: grid; gap: 8px; align-content: start; }
  .title { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .meta { display: flex; gap: 10px; color: var(--text-2); font-size: 12.5px; }
  .meta span + span::before { content: '·'; margin-right: 10px; color: var(--text-3); }
  .options { display: flex; gap: 12px; flex-wrap: wrap; margin-top: 4px; }
  .field { display: grid; gap: 4px; }
  .label { font-size: 12px; color: var(--text-3); }
  .static { padding: 8px 0; color: var(--text-2); }
  .segmented { display: inline-flex; background: var(--surface-2); border-radius: var(--radius-s); padding: 2px; }
  .segmented button { border: 0; background: transparent; color: var(--text-2); padding: 6px 12px; }
  .segmented button.on { background: var(--surface-3); color: var(--text); }
  .row { display: flex; gap: 8px; align-items: center; margin-top: 4px; }
  .hint { margin: 0; color: var(--text-2); font-size: 12.5px; }
  .empty { color: var(--text-3); margin: 0; }
  .nap { display: flex; align-items: center; gap: 14px; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-width: 760px; }
  .history-head { display: flex; align-items: baseline; justify-content: space-between; max-width: 760px; }
</style>
