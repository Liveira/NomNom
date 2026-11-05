<script lang="ts">
  import { onMount } from 'svelte';
  import Logo from '../lib/icons/Logo.svelte';
  import Listbox from '../lib/ui/Listbox.svelte';
  import { formatDuration } from '../lib/format';
  import { ipc, onQuickContext, quickIpc, type Analysis, type QuickContext } from '../lib/ipc';
  import { buildChoice, defaultsFrom, FALLBACK_DEFAULTS, formatOptions, qualityOptions, type QuickFormat } from './choice';

  let ctx = $state<QuickContext | null>(null);
  let analysis = $state<Analysis | null>(null);
  let phase = $state<'loading' | 'ready' | 'error'>('loading');
  let error = $state('');
  let defaults = $state(FALLBACK_DEFAULTS);
  let format = $state<QuickFormat>('video');
  let quality = $state<number | 'best'>('best');
  let sending = $state(false);
  let message = $state('');
  let duplicate = $state(false);
  let formatOpen = $state(false);
  let qualityOpen = $state(false);
  let run = 0;
  /** Bumped for every new link so the entrance animation plays again when a hidden popup is reused. */
  let epoch = $state(0);

  const info = $derived(analysis?.info ?? null);
  const qualities = $derived(qualityOptions(format, info));
  const formats = $derived(formatOptions(defaults));
  const listOpen = $derived(formatOpen || qualityOpen);

  /** Leave a neutral state behind, so a popup kept hidden never shows the previous link. */
  function closePopup() {
    run++;
    ctx = null;
    analysis = null;
    phase = 'loading';
    message = '';
    duplicate = false;
    sending = false;
    format = defaults.kind;
    quality = 'best';
    void quickIpc.close();
  }

  async function load() {
    const mine = ++run;
    epoch++;
    sending = false;
    duplicate = false;
    message = '';
    format = defaults.kind;
    quality = 'best';
    analysis = null;
    phase = 'loading';
    // The type and formats from Settings; if they can't be read, plain video as-is still works.
    const [settings, context] = await Promise.all([
      ipc.getSettings().catch(() => null),
      quickIpc.context(),
    ]);
    if (mine !== run) return;
    defaults = defaultsFrom(settings);
    format = defaults.kind;
    ctx = context;
    if (!ctx) return;
    // Don't wait for the lookup: the popup is useful (Enter works) before it finishes.
    void analyze(ctx.url, mine);
  }

  async function analyze(url: string, mine: number) {
    try {
      const result = await ipc.analyze(url);
      if (mine !== run) return;
      analysis = result;
      phase = 'ready';
      if (result.info.heights.length === 0) format = 'audio'; // nothing to see, only sound
    } catch (e) {
      if (mine !== run) return;
      error = String(e);
      phase = 'error';
    }
  }

  function setFormat(f: QuickFormat) {
    format = f;
    quality = 'best';
  }

  async function confirm(again = false) {
    if (sending || !ctx || phase === 'error') return;
    sending = true;
    try {
      const out = await ipc.enqueue(ctx.url, buildChoice(format, quality, defaults), info?.title, again);
      if (out.outcome === 'queued') {
        message = 'Downloading';
        setTimeout(() => closePopup(), 450);
      } else if (out.outcome === 'already_active') {
        message = 'Already in your queue';
        setTimeout(() => closePopup(), 900);
      } else {
        message = 'You already downloaded this. Press Enter to download it again.';
        duplicate = true;
        sending = false;
      }
    } catch (e) {
      message = String(e);
      sending = false;
    }
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.defaultPrevented) return;
    if (e.key === 'Escape') {
      e.preventDefault();
      closePopup();
    } else if (e.key === 'Enter' && !listOpen) {
      e.preventDefault();
      void confirm(duplicate);
    }
  }

  onMount(() => {
    void load().then(() => {
      // A freshly created window shows itself after its first paint, so it never flashes empty.
      // A pre-created hidden one has no link yet; the app shows it when the shortcut is pressed.
      if (ctx) requestAnimationFrame(() => requestAnimationFrame(() => void quickIpc.ready()));
    });
    const unlisten = onQuickContext(() => void load());
    return () => void unlisten.then((fn) => fn());
  });
</script>

<svelte:window onkeydown={onKeydown} />

<div class="frame">
  {#key epoch}
  <div class="card" role="dialog" aria-label="Quick download">
    <header>
      <Logo size={20} />
      <span class="brand">NomNom</span>
      <span class="site">{ctx?.site ?? ''}</span>
    </header>

    <div class="title" aria-live="polite">
      {#if phase === 'loading'}
        <div class="skeleton" aria-hidden="true"></div>
        <div class="hint">Looking up this link…</div>
      {:else if phase === 'error'}
        <div class="bad" role="alert">{error}</div>
      {:else if info}
        <div class="name" title={info.title}>{info.title}</div>
        <div class="hint">
          {#if info.uploader}{info.uploader}{/if}
          {#if info.duration_secs}<span class="tabular"> · {formatDuration(info.duration_secs)}</span>{/if}
        </div>
      {/if}
    </div>

    <div class="fields">
      <div class="field">
        <span class="label">Format</span>
        <Listbox label="Format" options={formats} value={format} onchange={setFormat} bind:open={formatOpen} />
      </div>
      <div class="field">
        <span class="label">Quality</span>
        <Listbox
          label="Quality"
          options={qualities}
          value={quality}
          onchange={(v) => (quality = v)}
          bind:open={qualityOpen}
          disabled={qualities.length === 1 && phase === 'loading'}
        />
      </div>
    </div>

    <footer>
      <span class="message" class:bad={!!message && !sending && !duplicate} role="status">
        {#if message}{message}{:else}<kbd>Esc</kbd> to close{/if}
      </span>
      <button class="go" onclick={() => confirm(duplicate)} disabled={sending || phase === 'error'}>
        {duplicate ? 'Download again' : 'Download'} <span aria-hidden="true">↵</span>
      </button>
    </footer>
  </div>
  {/key}
</div>

<style>
  :global(html), :global(body) { background: transparent; }
  .frame { height: 100%; padding: 16px; display: flex; align-items: flex-start; }
  .card {
    width: 100%; display: grid; gap: 14px; padding: 16px 18px 14px;
    background: var(--surface-1); border: 1px solid var(--border-strong); border-radius: var(--radius-l);
    box-shadow: 0 16px 40px rgba(0, 0, 0, 0.5), 0 2px 8px rgba(0, 0, 0, 0.3);
    animation: enter var(--dur-2) var(--ease-out);
  }
  @keyframes enter { from { opacity: 0; transform: translateY(6px) scale(0.98); } to { opacity: 1; transform: none; } }
  header { display: flex; align-items: center; gap: 8px; }
  .brand { font-family: var(--font-display); font-weight: 700; letter-spacing: 0.05em; font-size: 12px; color: var(--text-2); }
  .site { margin-left: auto; color: var(--text-2); font-size: 12.5px; }
  .title { min-height: 54px; display: grid; gap: 4px; align-content: start; }
  .name { font-family: var(--font-display); font-weight: 600; font-size: 16px; line-height: 1.3; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .hint { color: var(--text-3); font-size: 12.5px; }
  .bad { color: var(--danger); }
  .skeleton { height: 18px; width: 75%; border-radius: 4px; background: linear-gradient(90deg, var(--surface-3) 30%, var(--surface-2) 50%, var(--surface-3) 70%); background-size: 200% 100%; animation: shimmer 1.2s var(--ease-out) 1; }
  @keyframes shimmer { from { background-position: 100% 0; } to { background-position: -100% 0; } }
  .fields { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; }
  .field { display: grid; gap: 4px; }
  .label { font-size: 12px; color: var(--text-3); }
  footer { display: flex; align-items: center; justify-content: space-between; gap: 12px; }
  .message { color: var(--text-3); font-size: 12.5px; }
  kbd { font: inherit; padding: 1px 6px; border-radius: 4px; background: var(--surface-3); color: var(--text-2); }
  .go {
    display: inline-flex; align-items: center; gap: 8px; padding: 8px 16px; border: 0; cursor: pointer;
    border-radius: var(--radius-s); background: var(--accent); color: var(--on-accent); font-weight: 600;
    transition: background var(--dur-1), transform var(--dur-1);
  }
  .go:hover:not(:disabled) { background: var(--accent-hover); }
  .go:active:not(:disabled) { transform: scale(0.97); }
  .go:disabled { opacity: 0.55; cursor: default; }
</style>
