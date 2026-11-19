<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import {
    base64Decode, base64Encode, contrast, convertCase, countText, diffLines, digest, hslToRgb, jsonFormat, jsonMinify, md5,
    parseColor, parseMoment, rgbToHsl, toHex, urlDecode, urlEncode, type Case, type DiffLine, type Match, type Result,
  } from './logic';
  import { runRegex } from './runRegex';

  const TABS = [
    { id: 'json', label: 'JSON' },
    { id: 'base64', label: 'Base64' },
    { id: 'url', label: 'URL' },
    { id: 'hash', label: 'Hashes' },
    { id: 'uuid', label: 'UUID' },
    { id: 'regex', label: 'Regex' },
    { id: 'time', label: 'Timestamps' },
    { id: 'diff', label: 'Differences' },
    { id: 'colour', label: 'Colours' },
    { id: 'words', label: 'Words' },
  ] as const;
  type Tab = (typeof TABS)[number]['id'];

  let tab = $state<Tab>('json');
  let copied = $state('');

  async function copy(text: string, what: string) {
    await navigator.clipboard.writeText(text).catch(() => {});
    copied = what;
    setTimeout(() => (copied = ''), 1400);
  }

  // JSON
  let json = $state('');
  let jsonOut = $state<Result<string> | null>(null);
  // Base64 and URL
  let b64 = $state('');
  let b64Out = $state<Result<string> | null>(null);
  let url = $state('');
  let urlOut = $state<Result<string> | null>(null);
  // Hashes
  let hashText = $state('');
  let hashes = $state<{ name: string; value: string }[]>([]);
  $effect(() => {
    const t = hashText;
    void (async () => {
      if (!t) return void (hashes = []);
      const out = [{ name: 'MD5', value: md5(t) }];
      for (const a of ['SHA-1', 'SHA-256', 'SHA-384', 'SHA-512'] as const) out.push({ name: a, value: await digest(a, t) });
      if (t === hashText) hashes = out;
    })();
  });
  // UUID
  let uuidCount = $state(5);
  let uuids = $state<string[]>([]);
  const makeUuids = () => (uuids = Array.from({ length: Math.min(Math.max(uuidCount, 1), 100) }, () => crypto.randomUUID()));
  // Regex
  let pattern = $state('(\\d+)-(\\w+)');
  let flags = $state('g');
  let sample = $state('order 12-abc and 7-xyz');
  let found = $state<Result<Match[]> | null>(null);
  let regexToken = 0;
  $effect(() => {
    const [p, f, s] = [pattern, flags, sample];
    const mine = ++regexToken;
    if (!p) return void (found = null);
    void runRegex(p, f, s).then((r) => {
      if (mine === regexToken) found = r;
    });
  });
  // Timestamps
  let moment = $state(String(Math.floor(Date.now() / 1000)));
  const when = $derived(parseMoment(moment));
  // Differences
  let left = $state('');
  let right = $state('');
  const diff = $derived<Result<DiffLine[]>>(diffLines(left, right));
  // Colours
  let colour = $state('#ffa970');
  const rgb = $derived(parseColor(colour));
  const hsl = $derived(rgb ? rgbToHsl(rgb) : null);
  // Words
  let words = $state('');
  const counts = $derived(countText(words));
  const CASES: { id: Case; label: string }[] = [
    { id: 'upper', label: 'UPPER CASE' }, { id: 'lower', label: 'lower case' }, { id: 'title', label: 'Title Case' },
    { id: 'camel', label: 'camelCase' }, { id: 'snake', label: 'snake_case' }, { id: 'kebab', label: 'kebab-case' },
  ];

  function pick(e: KeyboardEvent) {
    const i = TABS.findIndex((t) => t.id === tab);
    if (e.key === 'ArrowRight') tab = TABS[(i + 1) % TABS.length].id;
    else if (e.key === 'ArrowLeft') tab = TABS[(i + TABS.length - 1) % TABS.length].id;
    else return;
    e.preventDefault();
  }
</script>

<header class="head">
  <h1><Icon name="wrench" size={24} /> Text toolbox</h1>
  <p class="sub">Small jobs you would otherwise search the web for. Everything happens on this computer, and nothing you type is sent anywhere.</p>
</header>

<div class="tabs" role="tablist" aria-label="Tool" tabindex="-1" onkeydown={pick}>
  {#each TABS as t (t.id)}
    <button role="tab" aria-selected={tab === t.id} tabindex={tab === t.id ? 0 : -1} class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
  {/each}
</div>

<section class="pane" role="tabpanel">
  {#if tab === 'json'}
    <textarea rows="9" bind:value={json} placeholder={'{"name": "NomNom"}'} aria-label="JSON" spellcheck="false" translate="no"></textarea>
    <div class="row">
      <button class="plain" onclick={() => (jsonOut = jsonFormat(json))}>Make readable</button>
      <button class="plain" onclick={() => (jsonOut = jsonMinify(json))}>Minify</button>
      {#if jsonOut?.ok}<button class="plain" onclick={() => copy(jsonOut?.ok ? jsonOut.value : '', 'json')}>{copied === 'json' ? 'Copied' : 'Copy result'}</button>{/if}
    </div>
    {#if jsonOut}
      {#if jsonOut.ok}<pre class="out" translate="no">{jsonOut.value}</pre>{:else}<p class="bad" role="alert">{jsonOut.error}</p>{/if}
    {/if}
  {:else if tab === 'base64'}
    <textarea rows="6" bind:value={b64} placeholder="Text or Base64" aria-label="Text or Base64" spellcheck="false" translate="no"></textarea>
    <div class="row">
      <button class="plain" onclick={() => (b64Out = { ok: true, value: base64Encode(b64) })}>Encode</button>
      <button class="plain" onclick={() => (b64Out = base64Decode(b64))}>Decode</button>
      {#if b64Out?.ok}<button class="plain" onclick={() => copy(b64Out?.ok ? b64Out.value : '', 'b64')}>{copied === 'b64' ? 'Copied' : 'Copy result'}</button>{/if}
    </div>
    {#if b64Out}{#if b64Out.ok}<pre class="out" translate="no">{b64Out.value}</pre>{:else}<p class="bad" role="alert">{b64Out.error}</p>{/if}{/if}
  {:else if tab === 'url'}
    <textarea rows="5" bind:value={url} placeholder="Text or an encoded address" aria-label="Text or encoded address" spellcheck="false" translate="no"></textarea>
    <div class="row">
      <button class="plain" onclick={() => (urlOut = { ok: true, value: urlEncode(url) })}>Encode</button>
      <button class="plain" onclick={() => (urlOut = urlDecode(url))}>Decode</button>
      {#if urlOut?.ok}<button class="plain" onclick={() => copy(urlOut?.ok ? urlOut.value : '', 'url')}>{copied === 'url' ? 'Copied' : 'Copy result'}</button>{/if}
    </div>
    {#if urlOut}{#if urlOut.ok}<pre class="out" translate="no">{urlOut.value}</pre>{:else}<p class="bad" role="alert">{urlOut.error}</p>{/if}{/if}
  {:else if tab === 'hash'}
    <textarea rows="5" bind:value={hashText} placeholder="Type or paste text" aria-label="Text to hash" spellcheck="false" translate="no"></textarea>
    <p class="hint">For a file's checksum, use a tool made for files. MD5 and SHA-1 are only for matching published checksums; they are not safe for anything secret.</p>
    {#each hashes as h (h.name)}
      <div class="line"><strong>{h.name}</strong><code translate="no">{h.value}</code><button class="plain small" onclick={() => copy(h.value, h.name)}>{copied === h.name ? 'Copied' : 'Copy'}</button></div>
    {/each}
  {:else if tab === 'uuid'}
    <div class="row">
      <label class="inline">How many <input type="number" min="1" max="100" bind:value={uuidCount} /></label>
      <button class="plain" onclick={makeUuids}>Make</button>
      {#if uuids.length}<button class="plain" onclick={() => copy(uuids.join('\n'), 'uuid')}>{copied === 'uuid' ? 'Copied' : 'Copy all'}</button>{/if}
    </div>
    {#each uuids as u (u)}<div class="line"><code translate="no">{u}</code><button class="plain small" onclick={() => copy(u, u)}>{copied === u ? 'Copied' : 'Copy'}</button></div>{/each}
  {:else if tab === 'regex'}
    <div class="row">
      <label class="grow">Pattern <input bind:value={pattern} spellcheck="false" translate="no" /></label>
      <label>Flags <input class="flags" bind:value={flags} maxlength="6" spellcheck="false" translate="no" /></label>
    </div>
    <textarea rows="5" bind:value={sample} aria-label="Text to test" spellcheck="false" translate="no"></textarea>
    {#if found}
      {#if found.ok}
        <p class="hint">{found.value.length} {found.value.length === 1 ? 'match' : 'matches'}</p>
        {#each found.value as m, i (i)}
          <div class="line"><code translate="no">{m.text}</code><span class="hint">at {m.index}</span>{#each m.groups as g, k (k)}<span class="chip" translate="no">${k + 1} = {g}</span>{/each}</div>
        {/each}
      {:else}<p class="bad" role="alert">{found.error}</p>{/if}
    {/if}
  {:else if tab === 'time'}
    <label class="grow">A Unix time (seconds or milliseconds) or a date
      <input bind:value={moment} spellcheck="false" translate="no" />
    </label>
    <div class="row"><button class="plain" onclick={() => (moment = String(Math.floor(Date.now() / 1000)))}>Now</button></div>
    {#if when}
      <div class="line"><strong>UTC</strong><code translate="no">{when.toISOString()}</code><button class="plain small" onclick={() => copy(when.toISOString(), 'iso')}>{copied === 'iso' ? 'Copied' : 'Copy'}</button></div>
      <div class="line"><strong>Local</strong><code translate="no">{when.toLocaleString()}</code></div>
      <div class="line"><strong>Seconds</strong><code translate="no">{Math.floor(when.getTime() / 1000)}</code><button class="plain small" onclick={() => copy(String(Math.floor(when.getTime() / 1000)), 's')}>{copied === 's' ? 'Copied' : 'Copy'}</button></div>
      <div class="line"><strong>Milliseconds</strong><code translate="no">{when.getTime()}</code><button class="plain small" onclick={() => copy(String(when.getTime()), 'ms')}>{copied === 'ms' ? 'Copied' : 'Copy'}</button></div>
    {:else if moment.trim()}<p class="bad" role="alert">That is not a time or a date I can read.</p>{/if}
  {:else if tab === 'diff'}
    <div class="two">
      <textarea rows="9" bind:value={left} placeholder="Original" aria-label="Original text" spellcheck="false" translate="no"></textarea>
      <textarea rows="9" bind:value={right} placeholder="Changed" aria-label="Changed text" spellcheck="false" translate="no"></textarea>
    </div>
    {#if diff.ok}
      {#if left || right}
        <pre class="out diff" translate="no">{#each diff.value as l, i (i)}<span class={l.kind}>{l.kind === 'add' ? '+ ' : l.kind === 'del' ? '- ' : '  '}{l.text}
</span>{/each}</pre>
      {/if}
    {:else}<p class="bad" role="alert">{diff.error}</p>{/if}
  {:else if tab === 'colour'}
    <label class="grow">A colour (#ffa970, rgb(255, 169, 112) or hsl(25, 100%, 72%))
      <input bind:value={colour} spellcheck="false" translate="no" />
    </label>
    {#if rgb && hsl}
      <div class="swatch" style="background: {toHex(rgb)}" role="img" aria-label="The colour"></div>
      <div class="line"><strong>HEX</strong><code translate="no">{toHex(rgb)}</code><button class="plain small" onclick={() => copy(toHex(rgb), 'hex')}>{copied === 'hex' ? 'Copied' : 'Copy'}</button></div>
      <div class="line"><strong>RGB</strong><code translate="no">rgb({rgb.r}, {rgb.g}, {rgb.b})</code><button class="plain small" onclick={() => copy(`rgb(${rgb.r}, ${rgb.g}, ${rgb.b})`, 'rgb')}>{copied === 'rgb' ? 'Copied' : 'Copy'}</button></div>
      <div class="line"><strong>HSL</strong><code translate="no">hsl({hsl.h}, {hsl.s}%, {hsl.l}%)</code><button class="plain small" onclick={() => copy(`hsl(${hsl.h}, ${hsl.s}%, ${hsl.l}%)`, 'hsl')}>{copied === 'hsl' ? 'Copied' : 'Copy'}</button></div>
      <div class="line"><strong>Against white</strong><code translate="no">{contrast(rgb, { r: 255, g: 255, b: 255 }).toFixed(2)} : 1</code></div>
      <div class="line"><strong>Against black</strong><code translate="no">{contrast(rgb, { r: 0, g: 0, b: 0 }).toFixed(2)} : 1</code></div>
      <p class="hint">Text needs 4.5 : 1 to be easy to read.</p>
    {:else if colour.trim()}<p class="bad" role="alert">That is not a colour I can read.</p>{/if}
  {:else}
    <textarea rows="7" bind:value={words} placeholder="Type or paste text" aria-label="Text" spellcheck="false"></textarea>
    <p class="hint"><span class="tabular">{counts.chars}</span> characters · <span class="tabular">{counts.words}</span> words · <span class="tabular">{counts.lines}</span> lines</p>
    <div class="row wrap">
      {#each CASES as c (c.id)}
        <button class="plain" disabled={!words} onclick={() => (words = convertCase(words, c.id))}>{c.label}</button>
      {/each}
      <button class="plain" disabled={!words} onclick={() => copy(words, 'words')}>{copied === 'words' ? 'Copied' : 'Copy'}</button>
    </div>
  {/if}
</section>

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .tabs { display: flex; flex-wrap: wrap; gap: 4px; padding: 3px; border-radius: 18px; background: var(--surface-2); border: 1px solid var(--border); width: fit-content; max-width: 100%; margin-bottom: 16px; }
  .tabs button { border: 0; background: none; color: var(--text-2); padding: 6px 14px; border-radius: 99px; cursor: pointer; font: inherit; font-weight: 600; font-size: 13px; }
  .tabs button.on { background: var(--surface-3); color: var(--text); }
  .pane { display: grid; gap: 10px; max-width: 900px; }
  textarea, input { font: inherit; padding: 9px 12px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); box-sizing: border-box; min-width: 0; }
  textarea { width: 100%; resize: vertical; font-family: var(--font-mono); font-size: 13px; }
  button:focus-visible, input:focus-visible, textarea:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  button.plain { display: inline-flex; align-items: center; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.small { padding: 3px 10px; font-size: 12.5px; }
  button:disabled { opacity: 0.5; cursor: default; }
  .row { display: flex; gap: 8px; align-items: flex-end; }
  .row.wrap { flex-wrap: wrap; }
  label { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  label.grow { flex: 1; }
  label.inline { display: flex; align-items: center; gap: 8px; }
  label.inline input { width: 80px; }
  .flags { width: 80px; }
  .out { margin: 0; padding: 10px 12px; border-radius: var(--radius-xs); background: var(--surface-2); font: 13px/1.5 var(--font-mono); white-space: pre-wrap; overflow-wrap: anywhere; max-height: 380px; overflow: auto; }
  .diff .add { color: var(--ok); }
  .diff .del { color: var(--danger); }
  .line { display: flex; align-items: center; gap: 10px; padding: 7px 12px; border-radius: var(--radius-xs); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); flex-wrap: wrap; }
  .line strong { min-width: 96px; font-size: 13px; }
  .line code { flex: 1; min-width: 0; overflow-wrap: anywhere; font-family: var(--font-mono); font-size: 12.5px; }
  .chip { padding: 2px 9px; border-radius: 99px; background: var(--surface-3); font: 12px var(--font-mono); }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .swatch { height: 64px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); }
</style>
