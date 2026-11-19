<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { onSnippetsChanged, snippetIpc, type Snippet } from '../../lib/ipc';

  const VARIABLES: [string, string][] = [
    ['{date}', "today's date, like 2026-11-19"],
    ['{time}', 'the time, like 14:30'],
    ['{weekday}', 'the day of the week'],
    ['{year}', 'the year'],
    ['{month}', 'the month number'],
    ['{day}', 'the day of the month'],
    ['{clipboard}', 'whatever is on the clipboard'],
  ];

  let list = $state<Snippet[]>([]);
  let editing = $state<{ id: number | null; name: string; body: string } | null>(null);
  let shown = $state('');
  let error = $state('');
  let copied = $state<number | null>(null);

  async function load() {
    try {
      list = await snippetIpc.list();
    } catch (e) {
      error = String(e);
    }
  }
  void load();
  let stop: (() => void) | null = null;
  void onSnippetsChanged(() => void load()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  let token = 0;
  $effect(() => {
    const body = editing?.body ?? '';
    const mine = ++token;
    if (!body.trim()) {
      shown = '';
      return;
    }
    void snippetIpc.preview(body).then((t) => {
      if (mine === token) shown = t;
    });
  });

  async function save() {
    if (!editing) return;
    error = '';
    try {
      await snippetIpc.save(editing.id, editing.name, editing.body);
      editing = null;
      await load();
    } catch (e) {
      error = String(e);
    }
  }
  async function remove(s: Snippet) {
    await snippetIpc.remove(s.id).catch((e) => (error = String(e)));
    await load();
  }
  async function copy(s: Snippet) {
    try {
      await snippetIpc.use(s.id, false);
      copied = s.id;
      setTimeout(() => (copied = null), 1600);
    } catch (e) {
      error = String(e);
    }
  }
  function add(v: string) {
    if (editing) editing.body += v;
  }
</script>

<header class="head">
  <h1><Icon name="file-text" size={24} /> Snippets</h1>
  <p class="sub">Text you type again and again. Save it under a name, then find it in the search (Alt+Space from anywhere, or Ctrl+K here) and it is pasted where you were typing, with the date or the clipboard filled in.</p>
  <button class="primary" onclick={() => (editing = { id: null, name: '', body: '' })}><Icon name="plus" size={16} /> New snippet</button>
</header>

{#if error}<p class="bad" role="alert">{error}</p>{/if}

{#if editing}
  <section class="card editor">
    <label>Name<input bind:value={editing.name} maxlength="60" placeholder="Sign off" /></label>
    <label>Text<textarea bind:value={editing.body} rows="6" maxlength="20000" placeholder={'Best regards,\nYour name\n{date}'}></textarea></label>
    <div class="vars">
      <span class="hint">Add a value:</span>
      {#each VARIABLES as [v, what] (v)}<button type="button" class="chip" title={what} translate="no" onclick={() => add(v)}>{v}</button>{/each}
    </div>
    {#if shown}<div><span class="hint">What it will paste right now:</span><pre class="out" translate="no">{shown}</pre></div>{/if}
    <div class="row">
      <button class="ghost" onclick={() => (editing = null)}>Cancel</button>
      <button class="primary" onclick={() => void save()}>Save</button>
    </div>
  </section>
{/if}

{#if list.length === 0 && !editing}
  <p class="hint">No snippets yet. Try a sign-off, an address you type often, or a standard reply.</p>
{:else}
  <ul class="list">
    {#each list as s (s.id)}
      <li>
        <div class="body">
          <strong translate="no">{s.name}</strong>
          <span class="text" translate="no">{s.body}</span>
        </div>
        <div class="btns">
          <button class="ghost small" onclick={() => void copy(s)}>{copied === s.id ? 'Copied' : 'Copy'}</button>
          <button class="ghost small" onclick={() => (editing = { id: s.id, name: s.name, body: s.body })}>Edit</button>
          <button class="ghost small" aria-label={`Delete ${s.name}`} onclick={() => void remove(s)}><Icon name="trash" size={15} /></button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  button.primary, button.ghost { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.ghost:hover { background: var(--surface-3); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button.small { padding: 5px 11px; font-size: 12.5px; }
  button:focus-visible, input:focus-visible, textarea:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; justify-items: start; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .card { padding: 14px 16px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border-strong); margin-bottom: 16px; max-width: 760px; }
  .editor { display: grid; gap: 10px; }
  label { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  input, textarea { font: inherit; padding: 8px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); box-sizing: border-box; width: 100%; }
  textarea { resize: vertical; font-family: var(--font-mono); font-size: 13px; }
  .vars { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; }
  .chip { border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text-2); border-radius: 99px; padding: 3px 11px; font: 12px var(--font-mono); cursor: pointer; }
  .chip:hover { background: var(--surface-3); color: var(--text); }
  .out { margin: 4px 0 0; padding: 8px 10px; border-radius: var(--radius-xs); background: var(--surface-2); font: 13px/1.5 var(--font-mono); white-space: pre-wrap; overflow-wrap: anywhere; max-height: 160px; overflow: auto; }
  .row { display: flex; gap: 8px; justify-content: flex-end; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-width: 860px; }
  .list li { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: var(--radius-s); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); }
  .body { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .text { color: var(--text-3); font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .btns { display: flex; gap: 6px; align-items: center; }
</style>
