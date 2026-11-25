<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { onRemindersChanged, reminderIpc, type Reminder, type ReminderPreview } from '../../lib/ipc';
  import { EXAMPLES, SNOOZES, dueText } from './logic';

  let list = $state<Reminder[]>([]);
  let text = $state('');
  let preview = $state<ReminderPreview | null>(null);
  let error = $state('');
  let now = $state(Math.floor(Date.now() / 1000));

  const upcoming = $derived(list.filter((r) => !r.done));
  const went = $derived(list.filter((r) => r.done));

  async function load() {
    try {
      list = await reminderIpc.list();
    } catch (e) {
      error = String(e);
    }
  }
  void load();
  let stop: (() => void) | null = null;
  let gone = false;
  void onRemindersChanged(() => void load()).then((u) => {
    if (gone) u();
    else stop = u;
  });
  // The "in 12 min" words stay true.
  const tick = setInterval(() => (now = Math.floor(Date.now() / 1000)), 20_000);
  onDestroy(() => {
    gone = true;
    stop?.();
    clearInterval(tick);
  });

  let token = 0;
  $effect(() => {
    const t = text;
    const mine = ++token;
    if (!t.trim()) {
      preview = null;
      return;
    }
    void reminderIpc
      .preview(t)
      .then((p) => {
        if (mine === token) preview = p;
      })
      .catch(() => {
        if (mine === token) preview = null;
      });
  });

  async function add() {
    error = '';
    try {
      await reminderIpc.addText(text);
      text = '';
      await load();
    } catch (e) {
      error = String(e);
    }
  }
  async function remove(r: Reminder) {
    await reminderIpc.remove(r.id).catch((e) => (error = String(e)));
    await load();
  }
  async function snooze(r: Reminder, minutes: number) {
    await reminderIpc.snooze(r.id, minutes).catch((e) => (error = String(e)));
    await load();
  }
</script>

<header class="head">
  <h1><Icon name="bell" size={24} /> Reminders</h1>
  <p class="sub">Say what and when, in your own words. A card shows up in the corner of the screen when it is time, even if NomNom's window is closed.</p>
</header>

<form class="add" onsubmit={(e) => { e.preventDefault(); void add(); }}>
  <input bind:value={text} placeholder="in 20 minutes stretch" aria-label="Reminder" spellcheck="false" maxlength="300" />
  <button class="primary" type="submit" disabled={!preview}>Remind me</button>
</form>
{#if preview}
  <p class="ok" role="status">
    <Icon name="check" size={14} /> <strong translate="no">{preview.text}</strong> <span>·</span> <span>{dueText(preview.due_at, now)}</span>
    {#if preview.repeat}<span>·</span> <span>{preview.repeat === 'daily' ? 'repeats every day' : 'repeats every week'}</span>{/if}
  </p>
{:else if text.trim()}
  <p class="hint">Say when, for example: in 20 minutes, at 18:30, or tomorrow at 9.</p>
{:else}
  <p class="hint">Try:
    {#each EXAMPLES as ex (ex)}<button type="button" class="chip" translate="no" onclick={() => (text = ex)}>{ex}</button>{/each}
  </p>
{/if}
{#if error}<p class="bad" role="alert">{error}</p>{/if}

<h2>Coming up</h2>
{#if upcoming.length === 0}
  <p class="hint">Nothing is waiting.</p>
{:else}
  <ul class="list">
    {#each upcoming as r (r.id)}
      <li>
        <span class="ico"><Icon name="bell" size={17} /></span>
        <div class="body">
          <strong translate="no">{r.text}</strong>
          <span class="when"><span>{dueText(r.due_at, now)}</span>{#if r.repeat} <span>·</span> <span>{r.repeat === 'daily' ? 'repeats every day' : 'repeats every week'}</span>{/if}</span>
        </div>
        <div class="btns">
          {#each SNOOZES as s (s.minutes)}<button class="ghost small" onclick={() => void snooze(r, s.minutes)}>{s.label}</button>{/each}
          <button class="ghost small" aria-label="Delete this reminder" onclick={() => void remove(r)}><Icon name="trash" size={15} /></button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

{#if went.length > 0}
  <h2>Went off</h2>
  <ul class="list quiet">
    {#each went as r (r.id)}
      <li>
        <span class="ico"><Icon name="check" size={17} /></span>
        <div class="body"><strong translate="no">{r.text}</strong></div>
        <div class="btns">
          <button class="ghost small" onclick={() => void snooze(r, 10)}>Again in 10 min</button>
          <button class="ghost small" aria-label="Delete this reminder" onclick={() => void remove(r)}><Icon name="trash" size={15} /></button>
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  button.primary, button.ghost { display: inline-flex; align-items: center; gap: 6px; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.ghost:hover { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.small { padding: 5px 11px; font-size: 12.5px; }
  button:disabled { opacity: 0.5; cursor: default; }
  button:focus-visible, input:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  h2 { font-size: 15px; margin: 22px 0 8px; color: var(--text-2); }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .hint { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-top: 8px; }
  .ok { color: var(--ok); font-size: 13px; margin: 8px 0 0; display: flex; gap: 6px; align-items: center; flex-wrap: wrap; }
  .bad { color: var(--danger); font-size: 13px; }
  .add { display: flex; gap: 8px; max-width: 760px; }
  .add input { flex: 1; min-width: 0; font: inherit; font-size: 15px; padding: 10px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); }
  .chip { border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text-2); border-radius: 99px; padding: 3px 11px; font: inherit; font-size: 12.5px; cursor: pointer; }
  .chip:hover { background: var(--surface-3); color: var(--text); }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-width: 860px; }
  .list.quiet li { opacity: 0.7; }
  li { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-1); border: 1px solid var(--border); }
  .ico { flex: none; width: 34px; height: 34px; border-radius: 11px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); }
  .body { flex: 1; min-width: 0; display: grid; gap: 2px; }
  .body strong { overflow-wrap: anywhere; }
  .when { color: var(--text-3); font-size: 12.5px; display: flex; gap: 6px; flex-wrap: wrap; }
  .btns { display: flex; gap: 6px; align-items: center; flex-wrap: wrap; justify-content: flex-end; }
</style>
