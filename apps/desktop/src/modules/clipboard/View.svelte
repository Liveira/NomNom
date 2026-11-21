<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { clipIpc, onClipboardChanged } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { onDestroy } from 'svelte';
  import ClipList from './ClipList.svelte';
  import { KEEP_CHOICES } from './logic';

  const enabled = $derived(app.settings?.clipboard.enabled ?? false);
  const keepDays = $derived(app.settings?.clipboard.keep_days ?? 30);
  const accel = $derived(app.settings?.shortcuts.clipboard ?? 'Ctrl+Alt+V');
  let count = $state(0);
  let confirming = $state(false);

  async function refresh() {
    count = await clipIpc.count().catch(() => 0);
  }
  void refresh();
  let stop: (() => void) | null = null;
  void onClipboardChanged(() => void refresh()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  async function clear(keepPinned: boolean) {
    await clipIpc.clear(keepPinned);
    confirming = false;
    await refresh();
  }
</script>

<header class="head">
  <div>
    <h1><Icon name="clipboard" size={24} /> Clipboard</h1>
    <p class="sub">Everything you copy, kept only on this computer, so you can find it and paste it again.</p>
  </div>
</header>

<section class="card settings">
  <div class="row">
    <div>
      <div class="label">Keep a history of what I copy</div>
      <div class="hint">Text only. Copies that a password manager marks as private are left out. Off until you turn it on.</div>
    </div>
    <Switch checked={enabled} label="Keep a history of what I copy" onchange={(v) => app.update({ clipboardEnabled: v })} />
  </div>
  <div class="row">
    <div>
      <div class="label">Forget things after</div>
      <div class="hint">Pinned items are never forgotten.</div>
    </div>
    <select aria-label="Forget things after" value={keepDays} onchange={(e) => app.update({ clipboardKeepDays: Number(e.currentTarget.value) })}>
      {#each KEEP_CHOICES as c (c.days)}<option value={c.days}>{c.label}</option>{/each}
    </select>
  </div>
  <div class="row">
    <div>
      <div class="label">Open it from anywhere</div>
      <div class="hint">Press <kbd>{accel}</kbd> to open the history next to what you are doing, choose an item, and it is pasted. You can change the keys in Settings, under Shortcuts.</div>
    </div>
    <button class="plain" disabled={!enabled} onclick={() => void clipIpc.open()}>Try it now</button>
  </div>
</section>

{#if enabled}
  <section class="card list">
    <div class="listhead">
      <h2>History <span class="count tabular">{count}</span></h2>
      {#if count > 0}
        {#if confirming}
          <span class="confirm">
            <button class="plain" onclick={() => clear(true)}>Forget all but pinned</button>
            <button class="plain danger" onclick={() => clear(false)}>Forget everything</button>
            <button class="plain" onclick={() => (confirming = false)}>Cancel</button>
          </span>
        {:else}
          <button class="plain" onclick={() => (confirming = true)}>Clear…</button>
        {/if}
      {/if}
    </div>
    <ClipList mode="page" />
  </section>
{:else}
  <p class="off">The history is off, so nothing is being saved.</p>
{/if}

<style>
  .head h1 { display: flex; align-items: center; gap: 10px; margin: 0 0 4px; }
  .sub { margin: 0 0 16px; color: var(--text-2); }
  .card { max-width: 760px; margin-bottom: 16px; padding: 16px 18px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border-strong); box-shadow: var(--shadow-card); }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .label { font-weight: 600; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  select { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; }
  kbd { font: inherit; padding: 1px 6px; border-radius: 5px; background: var(--surface-3); color: var(--text-2); border: 1px solid var(--border); }
  .listhead { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; }
  h2 { margin: 0; font-size: 16px; }
  .count { color: var(--text-3); font-size: 13px; margin-left: 6px; }
  .confirm { display: inline-flex; gap: 6px; flex-wrap: wrap; }
  button.plain { padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.danger { color: var(--danger); border-color: var(--danger); }
  .off { color: var(--text-3); }
</style>
