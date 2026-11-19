<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { isLinux } from '../../lib/platform';
  import { trIpc, type KeyStatus, type TrLanguage, type Translated } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { usageLine } from './logic';

  const enabled = $derived(app.settings?.text.enabled ?? false);
  const target = $derived(app.settings?.text.target ?? 'auto');
  const replaceKeys = $derived(app.settings?.shortcuts.translate_replace ?? 'Ctrl+Alt+T');
  const popupKeys = $derived(app.settings?.shortcuts.translate_popup ?? 'Ctrl+Alt+Y');

  let status = $state<KeyStatus | null>(null);
  let languages = $state<TrLanguage[]>([]);
  let keyText = $state('');
  let busy = $state(false);
  let error = $state('');
  let sample = $state('');
  let result = $state<Translated | null>(null);
  let trying = $state(false);

  async function load() {
    status = await trIpc.keyStatus().catch(() => null);
  }
  void load();
  void trIpc.languages().then((l) => (languages = l));

  async function saveKey() {
    busy = true;
    error = '';
    try {
      status = await trIpc.setKey(keyText);
      keyText = '';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function removeKey() {
    await trIpc.clearKey();
    await load();
  }
  async function tryIt() {
    trying = true;
    error = '';
    result = null;
    try {
      result = await trIpc.translate(sample);
    } catch (e) {
      error = String(e);
    } finally {
      trying = false;
    }
  }
</script>

<header class="head">
  <h1><Icon name="languages" size={24} /> Translator</h1>
  <p class="sub">Select text in any program and translate it with DeepL: put the translation in its place, or read it in a small window.</p>
</header>

<section class="card">
  <div class="row">
    <div>
      <div class="label">Translate selected text</div>
      <div class="hint">Turns on the two shortcuts below. NomNom copies the selection for a moment and puts your clipboard back after.</div>
    </div>
    <Switch checked={enabled} label="Translate selected text" onchange={(v) => app.update({ textEnabled: v })} />
  </div>

  <div class="row key">
    <div class="grow">
      <div class="label">Your DeepL key</div>
      {#if status?.set}
        <div class="hint ok">
          <Icon name="check" size={14} /> Key saved{status.free ? ' (free plan)' : ''}.
          {#if status.used !== null && status.limit !== null}{usageLine(status.used, status.limit)}{/if}
        </div>
      {:else}
        <div class="hint">Free keys come with a DeepL account and cover 500,000 characters a month. Get one at <span class="mono">deepl.com/pro-api</span>.</div>
      {/if}
      <form class="keyform" onsubmit={(e) => { e.preventDefault(); void saveKey(); }}>
        <input
          type="password"
          autocomplete="off"
          spellcheck="false"
          placeholder={status?.set ? 'Paste a new key to replace it' : 'Paste your key here'}
          aria-label="DeepL key"
          bind:value={keyText}
        />
        <button class="primary" type="submit" disabled={busy || keyText.trim().length < 10}>{busy ? 'Checking…' : 'Save and check'}</button>
        {#if status?.set}<button class="plain" type="button" onclick={removeKey}>Remove key</button>{/if}
      </form>
      <div class="hint">The key stays on this computer{isLinux ? ' (readable only by you)' : ', locked to your Windows account'}. NomNom sends only the text you choose to translate.</div>
    </div>
  </div>

  <div class="row">
    <div>
      <div class="label">Translate into</div>
      <div class="hint">Automatic follows the language NomNom is shown in.</div>
    </div>
    <select aria-label="Translate into" value={target} onchange={(e) => app.update({ textTarget: e.currentTarget.value })}>
      <option value="auto">Automatic</option>
      {#each languages as l (l.code)}<option value={l.code} translate="no">{l.name}</option>{/each}
    </select>
  </div>
</section>

<section class="card">
  <h2>Shortcuts</h2>
  <div class="row">
    <div>
      <div class="label">Replace the selection</div>
      <div class="hint">Select what you wrote, press <kbd>{replaceKeys}</kbd>, and it turns into the translation.{isLinux ? ' On Linux the translation is put on the clipboard for you to paste.' : ''}</div>
    </div>
  </div>
  <div class="row">
    <div>
      <div class="label">Show the translation</div>
      <div class="hint">Select any text, press <kbd>{popupKeys}</kbd>, and a small window shows it translated, with buttons to copy it or use it in place.</div>
    </div>
  </div>
  <p class="hint">You can change both in Settings, under Shortcuts.{isLinux ? ' Reading a selection on Linux needs wl-clipboard (Wayland) or xclip (X11) installed.' : ''}</p>
</section>

<section class="card">
  <h2>Try it</h2>
  <textarea rows="3" placeholder="Type something to translate" aria-label="Text to translate" bind:value={sample}></textarea>
  <div class="actions">
    <button class="primary" onclick={tryIt} disabled={trying || sample.trim() === '' || !status?.set}>{trying ? 'Translating…' : 'Translate'}</button>
  </div>
  {#if result}
    <div class="result" role="status">
      <div class="hint">From {result.detected || 'the detected language'}</div>
      <p translate="no">{result.text}</p>
    </div>
  {/if}
</section>

{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
  .head h1 { display: flex; align-items: center; gap: 10px; margin: 0 0 4px; }
  .sub { margin: 0 0 16px; color: var(--text-2); max-width: 640px; }
  .card { max-width: 760px; margin-bottom: 16px; padding: 16px 18px; border-radius: var(--radius-m); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-card); }
  h2 { margin: 0 0 8px; font-size: 16px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .row.key { align-items: flex-start; }
  .grow { flex: 1; min-width: 0; }
  .label { font-weight: 600; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .hint.ok { color: var(--ok); display: flex; align-items: center; gap: 6px; flex-wrap: wrap; }
  .mono { font-family: var(--font-mono); background: var(--surface-2); padding: 0 4px; border-radius: 4px; user-select: text; }
  .keyform { display: flex; gap: 8px; margin: 8px 0 4px; flex-wrap: wrap; }
  input[type='password'], textarea, select { background: var(--surface-2); color: var(--text); border: 1px solid var(--border-strong); border-radius: var(--radius-xs); padding: 8px 10px; font: inherit; }
  input[type='password'] { flex: 1; min-width: 220px; font-family: var(--font-mono); }
  textarea { width: 100%; box-sizing: border-box; resize: vertical; }
  .actions { display: flex; justify-content: flex-end; margin-top: 8px; }
  button.plain, button.primary { padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.5; cursor: default; }
  kbd { font: inherit; padding: 1px 6px; border-radius: 5px; background: var(--surface-3); color: var(--text-2); border: 1px solid var(--border); }
  .result { margin-top: 10px; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .result p { margin: 4px 0 0; white-space: pre-wrap; }
  .error { color: var(--danger); max-width: 760px; }
</style>
