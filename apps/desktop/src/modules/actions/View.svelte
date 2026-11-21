<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { isSuperKey, toAccelerator } from '../../lib/accelerator';
  import { actionsIpc, type ActionResult, type ActionView, type CustomAction } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import Dialog from '../../lib/ui/Dialog.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import { KINDS, blankAction, blankParam, missingValues, summary } from './logic';

  let list = $state<ActionView[]>([]);
  let error = $state('');
  /** The action being added or changed. */
  let editing = $state<CustomAction | null>(null);
  let formError = $state('');
  /** The action that is asking for values before it runs. */
  let asking = $state<CustomAction | null>(null);
  let values = $state<Record<string, string>>({});
  let result = $state<{ name: string; r: ActionResult } | null>(null);
  let busy = $state<string | null>(null);
  let recording = $state(false);
  let superHeld = false;

  async function load() {
    try {
      list = await actionsIpc.list();
    } catch (e) {
      error = String(e);
    }
  }
  /** The palette or the launcher sends actions that need values here. */
  function takePending() {
    try {
      const id = localStorage.getItem('nomnom.pendingAction');
      if (!id) return;
      localStorage.removeItem('nomnom.pendingAction');
      const a = list.find((x) => x.id === id);
      if (a) ask(a);
    } catch {
      /* nothing was waiting */
    }
  }
  void load().then(takePending);
  $effect(() => {
    window.addEventListener('nomnompending', takePending);
    return () => window.removeEventListener('nomnompending', takePending);
  });

  function ask(a: CustomAction) {
    values = Object.fromEntries(a.params.map((p) => [p.name, p.default]));
    asking = a;
  }

  async function run(a: CustomAction, given: Record<string, string> = {}) {
    busy = a.id;
    error = '';
    try {
      const r = await actionsIpc.run(a.id, given);
      if (a.kind === 'command' || !r.ok) result = { name: a.name, r };
    } catch (e) {
      error = String(e);
    } finally {
      busy = null;
    }
  }

  function start(a: CustomAction) {
    if (a.params.length > 0) ask(a);
    else void run(a);
  }

  async function submitAsk() {
    const a = asking;
    if (!a) return;
    asking = null;
    await run(a, { ...values });
  }

  async function save() {
    if (!editing) return;
    formError = '';
    try {
      list = await actionsIpc.save({ ...editing, args: kindUsesArgs(editing.kind) ? editing.args : '' });
      editing = null;
    } catch (e) {
      formError = String(e);
    }
  }

  async function remove(a: CustomAction) {
    try {
      list = await actionsIpc.remove(a.id);
    } catch (e) {
      error = String(e);
    }
  }

  const kindUsesArgs = (k: CustomAction['kind']) => k === 'app' || k === 'command';
  const kindInfo = (k: CustomAction['kind']) => KINDS.find((x) => x.id === k)!;

  async function browse(folder: boolean) {
    const picked = await open({ directory: folder, multiple: false });
    if (typeof picked === 'string' && editing) editing.target = picked;
  }
  async function browseParam(i: number) {
    if (!editing) return;
    const picked = await open({ directory: editing.params[i].kind === 'folder', multiple: false });
    if (typeof picked === 'string') values[editing.params[i].name] = picked;
  }

  function onKey(e: KeyboardEvent) {
    if (!recording || !editing) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      recording = false;
      return;
    }
    if (isSuperKey(e)) {
      superHeld = true;
      return;
    }
    const { accelerator } = toAccelerator({ code: e.code, ctrlKey: e.ctrlKey, altKey: e.altKey, shiftKey: e.shiftKey, metaKey: e.metaKey || superHeld });
    if (accelerator) {
      editing.shortcut = accelerator;
      recording = false;
    }
  }

  const missing = $derived(editing ? missingValues(editing) : []);
  function addValue(name: string) {
    editing?.params.push(blankParam(name));
  }
</script>

<svelte:window onstorage={takePending} />

<header class="head">
  <h1><Icon name="rocket" size={24} /> My actions</h1>
  <p class="sub">Programs, folders, links and commands of your own. Give one a shortcut and it works from anywhere, and every one shows up in the search (Ctrl+K inside NomNom, Alt+Space anywhere).</p>
  <button class="primary" onclick={() => { editing = blankAction(); formError = ''; }}><Icon name="plus" size={16} /> New action</button>
</header>

{#if error}<p class="bad" role="alert">{error}</p>{/if}

{#if list.length === 0}
  <section class="empty card">
    <Icon name="rocket" size={30} />
    <h2>Nothing here yet</h2>
    <p>Try a web search that asks what to look for, a folder you open every day, or a script you keep running by hand.</p>
    <button class="primary" onclick={() => { editing = blankAction(); formError = ''; }}>Make the first one</button>
  </section>
{:else}
  <ul class="list">
    {#each list as a (a.id)}
      <li>
        <span class="ico"><Icon name={kindInfo(a.kind).icon} size={18} /></span>
        <div class="body">
          <div class="r1"><strong translate="no">{a.name}</strong><span class="kind">{kindInfo(a.kind).label}</span>{#if a.params.length > 0}<span class="kind">asks for {a.params.length} {a.params.length === 1 ? 'value' : 'values'}</span>{/if}</div>
          <div class="cmd" translate="no" title={summary(a)}>{summary(a)}</div>
          {#if a.shortcut}
            <div class="key"><kbd>{a.shortcut}</kbd>{#if a.shortcut_error}<span class="bad" role="alert">{a.shortcut_error}</span>{/if}</div>
          {/if}
        </div>
        <div class="btns">
          <button class="primary small" disabled={busy === a.id} onclick={() => start(a)}>{busy === a.id ? 'Running…' : 'Run'}</button>
          <button class="ghost small" onclick={() => { editing = structuredClone($state.snapshot(a)); formError = ''; }}>Edit</button>
          <button class="ghost small" aria-label={`Delete ${a.name}`} onclick={() => void remove(a)}><Icon name="trash" size={15} /></button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

{#if editing}
  {@const info = kindInfo(editing.kind)}
  <Dialog title={editing.id ? 'Change action' : 'New action'} onclose={() => (editing = null)}>
    <div class="form">
      <label>Name<input bind:value={editing.name} maxlength="60" placeholder="Search the web" /></label>

      <div class="seg" role="radiogroup" aria-label="What it does">
        {#each KINDS as k (k.id)}
          <button type="button" role="radio" aria-checked={editing.kind === k.id} class:on={editing.kind === k.id} onclick={() => { if (editing) editing.kind = k.id; }}>{k.label}</button>
        {/each}
      </div>
      <p class="hint">{info.hint}</p>

      <label>{info.target}
        <span class="row">
          <input bind:value={editing.target} placeholder={editing.kind === 'link' ? 'https://example.com/?q={q}' : ''} spellcheck="false" translate="no" />
          {#if editing.kind !== 'link'}
            <button type="button" class="ghost small" onclick={() => void browse(false)}>File…</button>
            <button type="button" class="ghost small" onclick={() => void browse(true)}>Folder…</button>
          {/if}
        </span>
      </label>

      {#if kindUsesArgs(editing.kind)}
        <label>Arguments
          <input bind:value={editing.args} placeholder={'--file "{file}"'} spellcheck="false" translate="no" />
        </label>
        <label><span>Start in this folder <span class="opt">(optional)</span></span>
          <input bind:value={editing.cwd} placeholder="" spellcheck="false" translate="no" />
        </label>
      {/if}

      <fieldset>
        <legend>Values it asks for</legend>
        <p class="hint">Write <code>{'{name}'}</code> in the address or the arguments, and it asks for that value each time it runs from here or from the search. A shortcut uses the default.</p>
        {#each editing.params as p, i (i)}
          <div class="param">
            <input aria-label="Value name" bind:value={p.name} placeholder="name" spellcheck="false" translate="no" />
            <input aria-label="Question" bind:value={p.label} placeholder="Question to ask" />
            <input aria-label="Default" bind:value={p.default} placeholder="Default" spellcheck="false" translate="no" />
            <select aria-label="Kind of value" bind:value={p.kind}>
              <option value="text">Text</option>
              <option value="file">File</option>
              <option value="folder">Folder</option>
            </select>
            <button type="button" class="ghost small" aria-label="Remove this value" onclick={() => editing?.params.splice(i, 1)}><Icon name="x" size={14} /></button>
          </div>
        {/each}
        {#each missing as m (m)}
          <button type="button" class="ghost small" onclick={() => addValue(m)}><Icon name="plus" size={14} /> Add the value "{m}"</button>
        {/each}
        {#if missing.length === 0}
          <button type="button" class="ghost small" onclick={() => addValue('')}><Icon name="plus" size={14} /> Add a value</button>
        {/if}
      </fieldset>

      <div class="row split">
        <div>
          <div class="lbl"><span>Shortcut <span class="opt">(works from anywhere)</span></span></div>
          <span class="row">
            <button
              type="button"
              class="keybtn"
              class:recording
              onclick={() => (recording = true)}
              onkeydown={onKey}
              onkeyup={(e) => { if (isSuperKey(e)) superHeld = false; }}
              onblur={() => { recording = false; superHeld = false; }}
            >{recording ? 'Press the keys…' : (editing.shortcut ?? 'None')}</button>
            {#if editing.shortcut}<button type="button" class="ghost small" onclick={() => { if (editing) editing.shortcut = null; }}>Clear</button>{/if}
          </span>
          {#if recording}<div class="hint">Hold Ctrl, Alt, Shift or {isLinux ? 'Super' : 'Win'} and press a key. Esc cancels.</div>{/if}
        </div>
        <div class="sw">
          <span>Show in the search</span>
          <Switch checked={editing.in_palette} label="Show in the search" onchange={(v) => { if (editing) editing.in_palette = v; }} />
        </div>
      </div>
      {#if formError}<p class="bad" role="alert">{formError}</p>{/if}
    </div>
    {#snippet actions()}
      <button class="ghost" onclick={() => (editing = null)}>Cancel</button>
      <button class="primary" onclick={() => void save()}>Save</button>
    {/snippet}
  </Dialog>
{/if}

{#if asking}
  <Dialog title={asking.name} onclose={() => (asking = null)}>
    <form class="form" onsubmit={(e) => { e.preventDefault(); void submitAsk(); }}>
      {#each asking.params as p (p.name)}
        <label>{p.label || p.name}
          <span class="row">
            <input bind:value={values[p.name]} spellcheck="false" />
            {#if p.kind !== 'text'}
              <button type="button" class="ghost small" onclick={async () => { const v = await open({ directory: p.kind === 'folder', multiple: false }); if (typeof v === 'string') values[p.name] = v; }}>Choose…</button>
            {/if}
          </span>
        </label>
      {/each}
      <button type="submit" hidden aria-hidden="true" tabindex="-1">Run</button>
    </form>
    {#snippet actions()}
      <button class="ghost" onclick={() => (asking = null)}>Cancel</button>
      <button class="primary" onclick={() => void submitAsk()}>Run</button>
    {/snippet}
  </Dialog>
{/if}

{#if result}
  <Dialog title={result.name} onclose={() => (result = null)}>
    <p class:bad={!result.r.ok}>{result.r.message}</p>
    {#if result.r.output.trim()}<pre class="out" translate="no">{result.r.output}{result.r.truncated ? '\n…' : ''}</pre>{/if}
    {#snippet actions()}
      <button class="primary" onclick={() => (result = null)}>Close</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  button.primary, button.ghost { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.ghost:hover { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.small { padding: 5px 11px; font-size: 12.5px; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .head { display: grid; gap: 8px; margin-bottom: 18px; max-width: 760px; justify-items: start; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .empty { display: grid; justify-items: start; gap: 8px; padding: 24px; max-width: 560px; }
  .empty h2 { font-size: 18px; }
  .empty p { color: var(--text-2); margin: 0; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-width: 860px; }
  li { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-1); border: 1px solid var(--border); }
  .ico { flex: none; width: 36px; height: 36px; border-radius: 11px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); }
  .body { flex: 1; min-width: 0; display: grid; gap: 3px; }
  .r1 { display: flex; gap: 10px; align-items: baseline; flex-wrap: wrap; }
  .kind { color: var(--text-3); font-size: 12px; }
  .cmd { color: var(--text-3); font-size: 12px; font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .key { display: flex; gap: 8px; align-items: center; }
  kbd { font: 600 11px var(--font-mono); padding: 2px 7px; border-radius: 6px; border: 1px solid var(--border-strong); color: var(--text-2); }
  .btns { display: flex; gap: 6px; align-items: center; }
  .form { display: grid; gap: 12px; }
  label, .lbl { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  .opt { color: var(--text-3); font-weight: 500; }
  input, select { font: inherit; padding: 8px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); min-width: 0; }
  input:focus-visible, select:focus-visible { outline: 2px solid var(--focus); outline-offset: 1px; }
  .row { display: flex; gap: 8px; align-items: center; }
  .row input { flex: 1; }
  .split { justify-content: space-between; align-items: flex-end; }
  .sw { display: flex; gap: 10px; align-items: center; font-weight: 600; font-size: 13px; }
  .seg { display: inline-flex; flex-wrap: wrap; padding: 3px; border-radius: 99px; background: var(--surface-2); border: 1px solid var(--border); justify-self: start; }
  .seg button { border: 0; background: none; color: var(--text-2); padding: 5px 13px; border-radius: 99px; cursor: pointer; font-weight: 600; font-size: 12.5px; }
  .seg button.on { background: var(--surface-3); color: var(--text); }
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-s); padding: 10px 12px; display: grid; gap: 8px; }
  legend { font-weight: 700; font-size: 13px; padding: 0 6px; }
  .param { display: grid; grid-template-columns: 1fr 1.4fr 1fr 90px auto; gap: 6px; }
  code { font-family: var(--font-mono); background: var(--surface-3); padding: 1px 5px; border-radius: 5px; }
  .keybtn { font: 600 13px var(--font-mono); padding: 7px 12px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); cursor: pointer; }
  .keybtn.recording { border-color: var(--accent); }
  .out { margin: 8px 0 0; white-space: pre-wrap; word-break: break-word; font: 12.5px/1.5 var(--font-mono); max-height: 320px; overflow: auto; background: var(--surface-2); padding: 10px 12px; border-radius: var(--radius-xs); }
</style>
