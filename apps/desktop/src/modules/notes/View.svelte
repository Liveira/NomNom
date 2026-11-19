<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { noteIpc, onNotesChanged, type Note } from '../../lib/ipc';

  let notes = $state<Note[]>([]);
  let query = $state('');
  let draft = $state('');
  let editing = $state<{ id: number; text: string } | null>(null);
  let error = $state('');
  let copied = $state<number | null>(null);
  let box = $state<HTMLTextAreaElement | null>(null);

  async function load() {
    try {
      notes = await noteIpc.list(query);
    } catch (e) {
      error = String(e);
    }
  }
  let stop: (() => void) | null = null;
  onMount(() => {
    void load();
    void onNotesChanged(() => void load()).then((u) => (stop = u));
    // Ready to type, which is the point of the shortcut.
    box?.focus();
  });
  onDestroy(() => stop?.());

  let wait: ReturnType<typeof setTimeout> | undefined;
  $effect(() => {
    void query;
    clearTimeout(wait);
    wait = setTimeout(() => void load(), 160);
    return () => clearTimeout(wait);
  });

  async function add() {
    error = '';
    try {
      await noteIpc.save(null, draft);
      draft = '';
      await load();
      box?.focus();
    } catch (e) {
      error = String(e);
    }
  }
  async function saveEdit() {
    if (!editing) return;
    try {
      await noteIpc.save(editing.id, editing.text);
      editing = null;
      await load();
    } catch (e) {
      error = String(e);
    }
  }
  async function pin(n: Note) {
    await noteIpc.pin(n.id, !n.pinned).catch((e) => (error = String(e)));
    await load();
  }
  async function remove(n: Note) {
    await noteIpc.remove(n.id).catch((e) => (error = String(e)));
    await load();
  }
  async function copy(n: Note) {
    await navigator.clipboard.writeText(n.text).catch(() => {});
    copied = n.id;
    setTimeout(() => (copied = null), 1500);
  }
  function when(unix: number): string {
    return new Date(unix * 1000).toLocaleString(undefined, { day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
  }
  function typed(e: KeyboardEvent, save: () => void) {
    if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
      e.preventDefault();
      save();
    }
  }
</script>

<header class="head">
  <h1><Icon name="pencil" size={24} /> Notes</h1>
  <p class="sub">Short notes that stay on this computer. Press Ctrl+Enter to save. Type "note" and the text in the search (Ctrl+K) to jot one down from anywhere.</p>
</header>

<div class="new">
  <textarea
    bind:this={box}
    bind:value={draft}
    rows="3"
    placeholder="Write a note…"
    aria-label="New note"
    maxlength="20000"
    onkeydown={(e) => typed(e, () => void add())}
  ></textarea>
  <button class="primary" disabled={!draft.trim()} onclick={() => void add()}>Save note</button>
</div>
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<div class="find">
  <Icon name="search" size={16} />
  <input bind:value={query} placeholder="Search notes" aria-label="Search notes" />
</div>

{#if notes.length === 0}
  <p class="hint">{query ? 'No note has all of those words.' : 'No notes yet.'}</p>
{:else}
  <ul class="grid">
    {#each notes as n (n.id)}
      <li class:pinned={n.pinned}>
        {#if editing?.id === n.id}
          <textarea rows="5" bind:value={editing.text} aria-label="Edit note" onkeydown={(e) => typed(e, () => void saveEdit())}></textarea>
          <div class="btns">
            <button class="ghost small" onclick={() => (editing = null)}>Cancel</button>
            <button class="primary small" onclick={() => void saveEdit()}>Save</button>
          </div>
        {:else}
          <p class="text" translate="no">{n.text}</p>
          <div class="foot">
            <span class="when">{when(n.updated_at)}</span>
            <span class="btns">
              <button class="ghost small" aria-pressed={n.pinned} aria-label={n.pinned ? 'Unpin this note' : 'Pin this note'} onclick={() => void pin(n)}><Icon name="pin" size={14} /></button>
              <button class="ghost small" onclick={() => void copy(n)}>{copied === n.id ? 'Copied' : 'Copy'}</button>
              <button class="ghost small" onclick={() => (editing = { id: n.id, text: n.text })}>Edit</button>
              <button class="ghost small" aria-label="Delete this note" onclick={() => void remove(n)}><Icon name="trash" size={14} /></button>
            </span>
          </div>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  button.primary, button.ghost { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.ghost:hover { background: var(--surface-3); }
  button.ghost[aria-pressed='true'] { background: var(--glow-peach); color: var(--accent-text); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button.small { padding: 4px 10px; font-size: 12.5px; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:focus-visible, input:focus-visible, textarea:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .new { display: grid; gap: 8px; max-width: 760px; justify-items: end; }
  textarea { width: 100%; box-sizing: border-box; font: inherit; padding: 10px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); resize: vertical; }
  .find { display: flex; align-items: center; gap: 8px; max-width: 360px; margin: 18px 0 12px; padding: 0 12px; border: 1px solid var(--border-strong); border-radius: var(--radius-s); background: var(--surface-1); color: var(--text-3); }
  .find input { flex: 1; min-width: 0; border: 0; background: none; color: var(--text); font: inherit; padding: 9px 0; outline: none; }
  .grid { list-style: none; margin: 0; padding: 0; display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 10px; max-width: 1000px; }
  li { display: grid; gap: 8px; align-content: space-between; padding: 12px 14px; border-radius: var(--radius-s); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); }
  li.pinned { border-color: var(--border-glow); }
  .text { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; max-height: 220px; overflow: auto; }
  .foot { display: flex; justify-content: space-between; align-items: center; gap: 8px; flex-wrap: wrap; }
  .when { color: var(--text-3); font-size: 12px; }
  .btns { display: flex; gap: 6px; justify-content: flex-end; }
</style>
