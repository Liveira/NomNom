<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { pathIpc, type PathEntry, type PathView } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import Dialog from '../../lib/ui/Dialog.svelte';

  let view = $state<PathView | null>(null);
  let tab = $state<'user' | 'machine'>('user');
  /** The user's list being edited. */
  let items = $state<{ raw: string; expanded: string; issue: PathEntry['issue'] }[]>([]);
  let dirty = $state(false);
  let error = $state('');
  let note = $state('');
  let confirming = $state(false);
  let restoring = $state(false);

  function take(v: PathView) {
    view = v;
    items = v.user.map((e) => ({ ...e }));
    dirty = false;
  }
  void pathIpc.get().then(take).catch((e) => (error = String(e)));

  const problems = $derived(items.filter((i) => i.issue).length);
  const label = { duplicate: 'Repeated', missing: 'Not there', empty: 'Empty' } as const;

  function move(i: number, by: number) {
    const j = i + by;
    if (j < 0 || j >= items.length) return;
    [items[i], items[j]] = [items[j], items[i]];
    dirty = true;
  }
  function remove(i: number) {
    items.splice(i, 1);
    dirty = true;
  }
  function tidy() {
    items = items.filter((i) => !i.issue);
    dirty = true;
  }
  async function add() {
    const p = await open({ directory: true, multiple: false });
    if (typeof p === 'string' && !items.some((i) => i.raw.toLowerCase() === p.toLowerCase())) {
      items.push({ raw: p, expanded: p, issue: null });
      dirty = true;
    }
  }
  async function save() {
    confirming = false;
    error = '';
    try {
      take(await pathIpc.apply(items.map((i) => i.raw)));
      note = 'Saved. Programs you open from now on will see it.';
      setTimeout(() => (note = ''), 4000);
    } catch (e) {
      error = String(e);
    }
  }
  async function restore(name: string) {
    restoring = false;
    error = '';
    try {
      take(await pathIpc.restore(name));
      note = 'Put back.';
      setTimeout(() => (note = ''), 3000);
    } catch (e) {
      error = String(e);
    }
  }
  const when = (n: string) => {
    const m = /(\d{4})(\d{2})(\d{2})-(\d{2})(\d{2})(\d{2})/.exec(n);
    return m ? new Date(+m[1], +m[2] - 1, +m[3], +m[4], +m[5], +m[6]).toLocaleString() : n;
  };
</script>

<header class="head">
  <h1><Icon name="wrench" size={24} /> PATH editor</h1>
  <p class="sub">PATH is the list of folders Windows looks in when you type a program's name in a terminal. Here you can see yours, find the entries that repeat or point to folders that are gone, and tidy them. A copy of the old list is kept every time you save, so you can put it back.</p>
</header>

{#if isLinux}
  <p class="hint">The PATH editor works on Windows for now.</p>
{:else if view}
  <div class="tabs" role="tablist" aria-label="Whose list">
    <button role="tab" aria-selected={tab === 'user'} class:on={tab === 'user'} onclick={() => (tab = 'user')}>Yours</button>
    <button role="tab" aria-selected={tab === 'machine'} class:on={tab === 'machine'} onclick={() => (tab = 'machine')}>The whole computer's</button>
  </div>

  {#if tab === 'user'}
    <div class="bar">
      <span class="hint">{items.length} {items.length === 1 ? 'entry' : 'entries'}{problems > 0 ? `, ${problems} to look at` : ''}</span>
      <div class="row">
        <button class="plain" onclick={() => void add()}>Add a folder…</button>
        <button class="plain" disabled={problems === 0} onclick={tidy}>Remove the repeated and missing</button>
      </div>
    </div>
    <ul class="list">
      {#each items as it, i (i + it.raw)}
        <li class:issue={!!it.issue}>
          <div class="body">
            <span class="path" translate="no">{it.raw || '(empty)'}</span>
            {#if it.raw.includes('%')}<span class="hint" translate="no">{it.expanded}</span>{/if}
          </div>
          {#if it.issue}<span class="tag {it.issue}">{label[it.issue]}</span>{/if}
          <span class="mv">
            <button class="plain small" aria-label="Move up" disabled={i === 0} onclick={() => move(i, -1)}><Icon name="arrow-up" size={13} /></button>
            <button class="plain small" aria-label="Move down" disabled={i === items.length - 1} onclick={() => move(i, 1)}><Icon name="arrow-down" size={13} /></button>
            <button class="plain small" aria-label="Remove this entry" onclick={() => remove(i)}><Icon name="x" size={13} /></button>
          </span>
        </li>
      {/each}
    </ul>
    <div class="foot">
      <button class="plain" disabled={view.backups.length === 0} onclick={() => (restoring = true)}>Put back an older list…</button>
      <button class="primary" disabled={!dirty} onclick={() => (confirming = true)}>Save…</button>
    </div>
  {:else}
    <p class="hint">This list is shared by everyone who uses this computer. NomNom shows it but does not change it, because that needs administrator rights.</p>
    <ul class="list">
      {#each view.machine as it, i (i + it.raw)}
        <li class:issue={!!it.issue}>
          <div class="body"><span class="path" translate="no">{it.raw || '(empty)'}</span></div>
          {#if it.issue}<span class="tag {it.issue}">{label[it.issue]}</span>{/if}
        </li>
      {/each}
    </ul>
  {/if}
{/if}
{#if note}<p class="ok" role="status">{note}</p>{/if}
{#if error}<p class="bad" role="alert">{error}</p>{/if}

{#if confirming}
  <Dialog title="Save your PATH?" onclose={() => (confirming = false)}>
    <p>Your list will have {items.length} {items.length === 1 ? 'entry' : 'entries'}. The old list is saved first, so you can put it back from this page.</p>
    <p class="hint">Programs that are already open keep the old list until they are restarted.</p>
    {#snippet actions()}
      <button class="plain" onclick={() => (confirming = false)}>Cancel</button>
      <button class="primary" onclick={() => void save()}>Save</button>
    {/snippet}
  </Dialog>
{/if}

{#if restoring && view}
  <Dialog title="Put back an older list" onclose={() => (restoring = false)}>
    <ul class="list">
      {#each view.backups as b (b)}
        <li><span>{when(b)}</span><button class="plain small" onclick={() => void restore(b)}>Put back</button></li>
      {/each}
    </ul>
    {#snippet actions()}
      <button class="plain" onclick={() => (restoring = false)}>Close</button>
    {/snippet}
  </Dialog>
{/if}

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .bad { color: var(--danger); font-size: 13px; }
  .ok { color: var(--ok); font-size: 13px; }
  .tabs { display: flex; gap: 4px; padding: 3px; border-radius: 18px; background: var(--surface-2); border: 1px solid var(--border); width: fit-content; margin-bottom: 14px; }
  .tabs button { border: 0; background: none; color: var(--text-2); padding: 6px 16px; border-radius: 99px; cursor: pointer; font: inherit; font-weight: 600; font-size: 13px; }
  .tabs button.on { background: var(--surface-3); color: var(--text); }
  .bar { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; margin-bottom: 10px; max-width: 860px; }
  .row { display: flex; gap: 8px; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 5px; max-width: 860px; }
  .list li { display: flex; align-items: center; gap: 10px; padding: 7px 10px; border-radius: var(--radius-xs); background: var(--surface-1); border: 1px solid var(--border); }
  .list li.issue { border-color: var(--border-glow); }
  .body { flex: 1; min-width: 0; display: grid; }
  .path { font: 12.5px var(--font-mono); overflow-wrap: anywhere; }
  .tag { padding: 2px 10px; border-radius: 99px; font-size: 12px; font-weight: 700; background: var(--glow-peach); color: var(--accent-text); white-space: nowrap; }
  .tag.missing { background: color-mix(in srgb, var(--danger) 16%, transparent); color: var(--danger); }
  .mv { display: flex; gap: 4px; }
  .foot { display: flex; justify-content: space-between; margin-top: 14px; max-width: 860px; }
  button.plain, button.primary { display: inline-flex; align-items: center; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover:not(:disabled) { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.small { padding: 4px 8px; font-size: 12.5px; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
