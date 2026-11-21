<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { onDestroy } from 'svelte';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import { actionsIpc, ipc, onRulesChanged, rulesIpc, type ActionView, type FolderRule, type RuleLogItem, type RuleStep } from '../../lib/ipc';
  import Dialog from '../../lib/ui/Dialog.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import { STEP_KINDS, blankRule, describeRule, describeStep, leaf, newStep, parentOf, presets } from './logic';

  let list = $state<FolderRule[]>([]);
  let log = $state<RuleLogItem[]>([]);
  let myActions = $state<ActionView[]>([]);
  let downloads = $state('');
  let error = $state('');
  let editing = $state<FolderRule | null>(null);
  let extText = $state('');
  let formError = $state('');
  let preview = $state<{ rule: FolderRule; files: { name: string; size: number }[] } | null>(null);
  let busy = $state(false);
  let note = $state('');

  const actionName = (id: string) => myActions.find((a) => a.id === id)?.name ?? 'an action';

  async function load() {
    try {
      [list, log] = await Promise.all([rulesIpc.list(), rulesIpc.log()]);
    } catch (e) {
      error = String(e);
    }
  }
  void load();
  void actionsIpc.list().then((a) => (myActions = a)).catch(() => {});
  void ipc.defaultDownloadDir().then((d) => (downloads = d)).catch(() => {});
  let stop: (() => void) | null = null;
  void onRulesChanged(() => void load()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  function startNew(from?: FolderRule) {
    editing = from ?? blankRule(downloads);
    extText = (editing.extensions ?? []).join(', ');
    formError = '';
  }
  function edit(r: FolderRule) {
    startNew(structuredClone($state.snapshot(r)));
  }

  async function save() {
    if (!editing) return;
    formError = '';
    try {
      list = await rulesIpc.save({ ...editing, extensions: extText.split(/[\s,;]+/).filter(Boolean) });
      editing = null;
    } catch (e) {
      formError = String(e);
    }
  }
  async function toggle(r: FolderRule, enabled: boolean) {
    error = '';
    try {
      list = await rulesIpc.save({ ...r, enabled });
    } catch (e) {
      error = String(e);
    }
  }
  async function remove(r: FolderRule) {
    list = await rulesIpc.remove(r.id).catch((e) => {
      error = String(e);
      return list;
    });
  }

  async function check(r: FolderRule) {
    error = '';
    note = '';
    try {
      preview = { rule: r, files: await rulesIpc.preview(r.id) };
    } catch (e) {
      error = String(e);
    }
  }
  async function applyNow() {
    if (!preview) return;
    busy = true;
    try {
      const n = await rulesIpc.applyExisting(preview.rule.id);
      note = n === 1 ? '1 file handled.' : `${n} files handled.`;
      preview = null;
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  async function undo(l: RuleLogItem) {
    error = '';
    try {
      await rulesIpc.undo(l.id);
      await load();
    } catch (e) {
      error = String(e);
    }
  }

  async function browse(set: (p: string) => void) {
    const p = await open({ directory: true, multiple: false });
    if (typeof p === 'string') set(p);
  }

  const time = (at: number) => new Date(at * 1000).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
  const stepWord: Record<string, string> = { move: 'Moved', copy: 'Copied', rename: 'Renamed', convert: 'Converted', run: 'Ran an action on', error: 'Stopped on' };
  function shift(i: number, by: number) {
    if (!editing) return;
    const j = i + by;
    if (j < 0 || j >= editing.steps.length) return;
    [editing.steps[i], editing.steps[j]] = [editing.steps[j], editing.steps[i]];
  }
  function change(i: number, type: RuleStep['type']) {
    if (editing) editing.steps[i] = newStep(type);
  }
</script>

<header class="head">
  <h1><Icon name="wand" size={24} /> Folder rules</h1>
  <p class="sub">When a file shows up in a folder, NomNom can move it, rename it or turn a picture into WebP. It waits until the file has finished arriving, never replaces anything and never deletes anything, and each move can be put back.</p>
  <div class="row">
    <button class="primary" onclick={() => startNew()}><Icon name="plus" size={16} /> New rule</button>
  </div>
</header>

{#if error}<p class="bad" role="alert">{error}</p>{/if}
{#if note}<p class="ok" role="status">{note}</p>{/if}

{#if list.length === 0}
  <section class="empty card">
    <h2>Start from an example</h2>
    <p>Pick one, look at it, and change what you like. It only works on files that arrive from now on, unless you choose to apply it to what is there.</p>
    <ul class="presets">
      {#each presets(downloads || 'C:\\Users\\you\\Downloads') as p (p.name)}
        <li><button class="ghost" onclick={() => startNew(p.rule('', ''))}>{p.name}</button></li>
      {/each}
    </ul>
  </section>
{:else}
  <ul class="rules">
    {#each list as r (r.id)}
      <li class:off={!r.enabled}>
        <div class="body">
          <strong translate="no">{r.name}</strong>
          <span class="desc" translate="no">{describeRule(r, actionName)}</span>
          <span class="path" translate="no" title={r.folder}>{r.folder}</span>
        </div>
        <div class="btns">
          <button class="ghost small" onclick={() => void check(r)}>What is there now</button>
          <button class="ghost small" onclick={() => edit(r)}>Edit</button>
          <button class="ghost small" aria-label={`Delete ${r.name}`} onclick={() => void remove(r)}><Icon name="trash" size={15} /></button>
          <Switch checked={r.enabled} label={`Turn ${r.name} on or off`} onchange={(v) => void toggle(r, v)} />
        </div>
      </li>
    {/each}
  </ul>
{/if}

<h2 class="sect">What happened</h2>
{#if log.length === 0}
  <p class="hint">Nothing yet.</p>
{:else}
  <div class="row"><button class="ghost small" onclick={() => void rulesIpc.clearLog().then(load)}>Clear the list</button></div>
  <ul class="log">
    {#each log as l (l.id)}
      <li class:fail={!l.ok} class:undone={l.undone}>
        <span class="when tabular">{time(l.at)}</span>
        <span class="what" translate="no">
          <strong>{l.rule}</strong>:
          {#if l.ok}
            <span>{stepWord[l.step] ?? l.step}</span> <span>{leaf(l.from)}</span>
            {#if l.step === 'move' || l.step === 'copy'}<span class="arrow">→</span> <span>{leaf(parentOf(l.to))}</span>
            {:else if l.to !== l.from}<span class="arrow">→</span> <span>{leaf(l.to)}</span>{/if}
          {:else}
            {l.note}
          {/if}
        </span>
        {#if l.ok && (l.step === 'move' || l.step === 'rename') && !l.undone}
          <button class="ghost small" onclick={() => void undo(l)}>Put back</button>
        {:else if l.undone}<span class="hint">Put back</span>{/if}
      </li>
    {/each}
  </ul>
{/if}

{#if editing}
  <Dialog title={editing.id ? 'Change rule' : 'New rule'} onclose={() => (editing = null)}>
    <div class="form">
      <label>Name<input bind:value={editing.name} maxlength="60" placeholder="Pictures to WebP" /></label>
      <label>Watch this folder
        <span class="row">
          <input bind:value={editing.folder} spellcheck="false" translate="no" />
          <button type="button" class="ghost small" onclick={() => void browse((p) => { if (editing) editing.folder = p; })}>Choose…</button>
        </span>
      </label>
      <div class="inline"><span>Also look in the folders inside it</span><Switch checked={editing.subfolders} label="Also look in the folders inside it" onchange={(v) => { if (editing) editing.subfolders = v; }} /></div>

      <fieldset>
        <legend>Which files</legend>
        <label>Kinds of file <span class="opt">(for example png, jpg. Empty means any)</span><input bind:value={extText} spellcheck="false" translate="no" /></label>
        <label>The name has <span class="opt">(optional)</span><input bind:value={editing.name_has} translate="no" /></label>
        <div class="two">
          <label>At least (KB)<input type="number" min="0" bind:value={editing.min_kb} /></label>
          <label>At most (KB)<input type="number" min="0" bind:value={editing.max_kb} /></label>
        </div>
        <p class="hint">Files still downloading are never touched. Zero means no limit.</p>
      </fieldset>

      <fieldset>
        <legend>What to do, in order</legend>
        {#each editing.steps as s, i (i)}
          <div class="step">
            <div class="top">
              <select aria-label="What to do" value={s.type} onchange={(e) => change(i, (e.currentTarget as HTMLSelectElement).value as RuleStep['type'])}>
                {#each STEP_KINDS as k (k.type)}<option value={k.type}>{k.label}</option>{/each}
              </select>
              <span class="mv">
                <button type="button" class="ghost small" aria-label="Move this step up" disabled={i === 0} onclick={() => shift(i, -1)}><Icon name="arrow-up" size={13} /></button>
                <button type="button" class="ghost small" aria-label="Move this step down" disabled={i === editing.steps.length - 1} onclick={() => shift(i, 1)}><Icon name="arrow-down" size={13} /></button>
                <button type="button" class="ghost small" aria-label="Remove this step" onclick={() => editing?.steps.splice(i, 1)}><Icon name="x" size={13} /></button>
              </span>
            </div>
            {#if s.type === 'move' || s.type === 'copy'}
              <span class="row">
                <input aria-label="Folder" bind:value={s.folder} placeholder="Folder" spellcheck="false" translate="no" />
                <button type="button" class="ghost small" onclick={() => void browse((p) => { s.folder = p; })}>Choose…</button>
              </span>
            {:else if s.type === 'rename'}
              <input aria-label="New name" bind:value={s.template} placeholder={'{date}_{name}'} spellcheck="false" translate="no" />
            {:else if s.type === 'convert'}
              <span class="row">
                <select aria-label="Format" bind:value={s.to}><option value="webp">WebP</option><option value="jpg">JPG</option><option value="png">PNG</option></select>
                <input aria-label="Folder for the originals" bind:value={s.originals} placeholder="Originals go here (optional)" spellcheck="false" translate="no" />
                <button type="button" class="ghost small" onclick={() => void browse((p) => { s.originals = p; })}>Choose…</button>
              </span>
            {:else if s.type === 'run'}
              <select aria-label="Action" bind:value={s.action}>
                <option value="">Choose an action</option>
                {#each myActions as a (a.id)}<option value={a.id}>{a.name}</option>{/each}
              </select>
            {/if}
            <p class="hint">{STEP_KINDS.find((k) => k.type === s.type)?.hint}</p>
          </div>
        {/each}
        <button type="button" class="ghost small" disabled={editing.steps.length >= 6} onclick={() => editing?.steps.push(newStep('move'))}><Icon name="plus" size={14} /> Add a step</button>
      </fieldset>
      {#if formError}<p class="bad" role="alert">{formError}</p>{/if}
    </div>
    {#snippet actions()}
      <button class="ghost" onclick={() => (editing = null)}>Cancel</button>
      <button class="primary" onclick={() => void save()}>Save</button>
    {/snippet}
  </Dialog>
{/if}

{#if preview}
  <Dialog title={preview.rule.name} onclose={() => (preview = null)}>
    {#if preview.files.length === 0}
      <p>No file in that folder fits this rule right now.</p>
    {:else}
      <p>{preview.files.length === 1 ? '1 file' : `${preview.files.length} files`} in the folder fit this rule. If you apply it now, each one goes through: <span translate="no">{preview.rule.steps.map((s) => describeStep(s, actionName)).join(', then ')}</span>.</p>
      <ul class="files">
        {#each preview.files.slice(0, 100) as f (f.name)}<li><span translate="no">{f.name}</span><span class="hint tabular">{formatBytes(f.size)}</span></li>{/each}
      </ul>
      {#if preview.files.length > 100}<p class="hint">and more. At most 60 are handled at a time.</p>{/if}
    {/if}
    {#snippet actions()}
      <button class="ghost" onclick={() => (preview = null)}>Close</button>
      {#if preview && preview.files.length > 0}<button class="primary" disabled={busy} onclick={() => void applyNow()}>{busy ? 'Working…' : 'Apply now'}</button>{/if}
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
  button:focus-visible, input:focus-visible, select:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .head { display: grid; gap: 8px; margin-bottom: 14px; max-width: 760px; justify-items: start; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  h2 { font-size: 17px; margin: 0 0 4px; }
  h2.sect { margin: 26px 0 8px; font-size: 15px; color: var(--text-2); }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .ok { color: var(--ok); font-size: 13px; }
  .row { display: flex; gap: 8px; align-items: center; }
  .row input { flex: 1; min-width: 0; }
  .empty { display: grid; gap: 8px; padding: 18px 20px; max-width: 640px; }
  .empty p { margin: 0; color: var(--text-2); font-size: 13.5px; }
  .presets { list-style: none; margin: 4px 0 0; padding: 0; display: grid; gap: 6px; justify-items: start; }
  .rules, .log, .files { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-width: 900px; }
  .rules li { display: flex; align-items: center; gap: 12px; padding: 12px 14px; border-radius: var(--radius-s); background: var(--surface-1); border: 1px solid var(--border); }
  .rules li.off { opacity: 0.65; }
  .body { flex: 1; min-width: 0; display: grid; gap: 3px; }
  .desc { color: var(--text-2); font-size: 13px; }
  .path { color: var(--text-3); font: 12px var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .btns { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
  .log { gap: 4px; margin-top: 6px; }
  .log li { display: flex; gap: 12px; align-items: center; padding: 6px 10px; border-radius: var(--radius-xs); background: var(--surface-1); border: 1px solid var(--border); font-size: 13px; }
  .log li.fail .what { color: var(--danger); }
  .log li.undone { opacity: 0.55; }
  .arrow { color: var(--text-3); margin: 0 4px; }
  .when { color: var(--text-3); font-size: 12px; min-width: 110px; }
  .what { flex: 1; min-width: 0; overflow-wrap: anywhere; }
  .files { max-height: 260px; overflow: auto; gap: 3px; margin-top: 8px; }
  .files li { display: flex; justify-content: space-between; gap: 12px; padding: 4px 8px; border-radius: 6px; background: var(--surface-2); font-size: 13px; }
  .form { display: grid; gap: 12px; min-width: 0; }
  .form > *, fieldset > * { min-width: 0; }
  label { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  .opt { color: var(--text-3); font-weight: 500; }
  input, select { font: inherit; padding: 8px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); min-width: 0; box-sizing: border-box; }
  .inline { display: flex; justify-content: space-between; align-items: center; gap: 12px; font-weight: 600; font-size: 13px; }
  fieldset { border: 1px solid var(--border); border-radius: var(--radius-s); padding: 10px 12px; display: grid; gap: 8px; }
  legend { font-weight: 700; font-size: 13px; padding: 0 6px; }
  .two { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .step { display: grid; gap: 6px; padding-bottom: 8px; border-bottom: 1px solid var(--border); min-width: 0; }
  .top { display: flex; gap: 8px; align-items: center; justify-content: space-between; }
  .top select { flex: 1; min-width: 0; }
  .mv { display: flex; gap: 4px; }
</style>
