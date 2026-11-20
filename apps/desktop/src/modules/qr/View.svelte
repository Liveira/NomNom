<script lang="ts">
  import { save } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { qrIpc, type QrScanned } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';

  const TABS = [
    { id: 'text', label: 'From text' },
    { id: 'wifi', label: 'Wi-Fi' },
    { id: 'read', label: 'Read from the screen' },
  ] as const;
  let tab = $state<(typeof TABS)[number]['id']>('text');
  let text = $state('');
  let ssid = $state('');
  let password = $state('');
  let security = $state<'WPA' | 'WEP' | 'none'>('WPA');
  let hidden = $state(false);
  let svg = $state('');
  let inside = $state('');
  let error = $state('');
  let note = $state('');
  let found = $state<QrScanned[] | null>(null);
  let reading = $state(false);

  let token = 0;
  $effect(() => {
    const mine = ++token;
    const t = tab;
    const [a, b, c, d, e] = [text, ssid, password, security, hidden];
    if (t === 'read') return;
    if ((t === 'text' && !a.trim()) || (t === 'wifi' && !b.trim())) {
      svg = '';
      inside = '';
      error = '';
      return;
    }
    const job = t === 'text' ? qrIpc.make(a).then((s) => [s, a] as [string, string]) : qrIpc.wifi(b, c, d, e);
    job
      .then(([s, payload]) => {
        if (mine !== token) return;
        svg = s;
        inside = payload;
        error = '';
      })
      .catch((err) => {
        if (mine !== token) return;
        svg = '';
        inside = '';
        error = String(err);
      });
  });

  function say(t: string) {
    note = t;
    setTimeout(() => (note = ''), 2200);
  }
  async function saveSvg() {
    const path = await save({ defaultPath: 'qr-code.svg', filters: [{ name: 'SVG picture', extensions: ['svg'] }] });
    if (!path) return;
    try {
      await qrIpc.save(path, svg);
      say('Saved.');
    } catch (e) {
      error = String(e);
    }
  }
  async function copy(t: string) {
    await navigator.clipboard.writeText(t).catch(() => {});
    say('Copied.');
  }
  async function readScreen() {
    reading = true;
    error = '';
    found = null;
    try {
      found = await qrIpc.readScreen();
    } catch (e) {
      error = String(e);
    } finally {
      reading = false;
    }
  }
</script>

<header class="head">
  <h1><Icon name="layers" size={24} /> QR codes</h1>
  <p class="sub">Make a code from some text or a Wi-Fi network, to scan with a phone. Or read a code that is on your screen. Everything stays on this computer.</p>
</header>

<div class="tabs" role="tablist" aria-label="What to do">
  {#each TABS as t (t.id)}
    <button role="tab" aria-selected={tab === t.id} class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
  {/each}
</div>

<section class="pane" role="tabpanel">
  {#if tab === 'text'}
    <label>Text or a link<textarea bind:value={text} rows="4" maxlength="1800" placeholder="https://example.com" spellcheck="false" translate="no"></textarea></label>
  {:else if tab === 'wifi'}
    <label>Network name<input bind:value={ssid} maxlength="32" spellcheck="false" translate="no" /></label>
    <label>Password<input bind:value={password} maxlength="63" spellcheck="false" translate="no" disabled={security === 'none'} /></label>
    <div class="row">
      <label class="inline">Security
        <select bind:value={security}><option value="WPA">WPA or WPA2</option><option value="WEP">WEP</option><option value="none">None</option></select>
      </label>
      <label class="inline"><input type="checkbox" bind:checked={hidden} /> The network is hidden</label>
    </div>
  {:else if isLinux}
    <p class="hint">Reading a code from the screen works on Windows for now.</p>
  {:else}
    <p class="hint">Show the code on the screen, on the monitor where the pointer is, and press the button. NomNom looks at that screen once and does not keep the picture.</p>
    <div class="row"><button class="primary" disabled={reading} onclick={() => void readScreen()}>{reading ? 'Looking…' : 'Read a code from the screen'}</button></div>
    {#if found}
      {#if found.length === 0}<p class="hint">No QR code was found on that screen.</p>{/if}
      {#each found as f (f.text)}
        <div class="found">
          <strong>{f.kind}</strong>
          <pre translate="no">{f.text}</pre>
          <div class="row">
            <button class="plain" onclick={() => void copy(f.text)}>Copy</button>
            {#if f.openable}<button class="plain" onclick={() => void qrIpc.open(f.text).catch((e) => (error = String(e)))}>Open</button>{/if}
          </div>
        </div>
      {/each}
    {/if}
  {/if}

  {#if tab !== 'read' && svg}
    <div class="code">
      <div class="pic" role="img" aria-label="The QR code">{@html svg}</div>
      <div class="row">
        <button class="plain" onclick={() => void saveSvg()}>Save as a picture…</button>
        <button class="plain" onclick={() => void copy(inside)}>Copy the text inside</button>
      </div>
    </div>
  {/if}
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
  <p class="ok" role="status">{note}</p>
</section>

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; max-width: 640px; }
  .bad { color: var(--danger); font-size: 13px; margin: 0; }
  .ok { color: var(--ok); font-size: 13px; margin: 0; min-height: 1.2em; }
  .tabs { display: flex; gap: 4px; padding: 3px; border-radius: 18px; background: var(--surface-2); border: 1px solid var(--border); width: fit-content; max-width: 100%; flex-wrap: wrap; margin-bottom: 16px; }
  .tabs button { border: 0; background: none; color: var(--text-2); padding: 6px 14px; border-radius: 99px; cursor: pointer; font: inherit; font-weight: 600; font-size: 13px; }
  .tabs button.on { background: var(--surface-3); color: var(--text); }
  .pane { display: grid; gap: 12px; max-width: 640px; }
  label { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  label.inline { display: flex; align-items: center; gap: 8px; }
  input, textarea, select { font: inherit; padding: 8px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); box-sizing: border-box; min-width: 0; width: 100%; }
  input[type='checkbox'] { width: auto; }
  label.inline select { width: auto; }
  input:disabled { opacity: 0.5; }
  .row { display: flex; gap: 10px; align-items: center; flex-wrap: wrap; }
  .code { display: grid; gap: 10px; justify-items: start; }
  .pic { width: 260px; max-width: 100%; background: #fff; padding: 6px; border-radius: 10px; line-height: 0; }
  .pic :global(svg) { width: 100%; height: auto; }
  .found { padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-1); border: 1px solid var(--border-strong); display: grid; gap: 6px; }
  pre { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font: 13px/1.5 var(--font-mono); }
  button.plain, button.primary { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.6; cursor: default; }
  button:focus-visible, input:focus-visible, textarea:focus-visible, select:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
