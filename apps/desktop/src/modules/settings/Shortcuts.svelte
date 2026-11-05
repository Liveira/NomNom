<script lang="ts">
  import { toAccelerator } from '../../lib/accelerator';
  import { app } from '../../app/state.svelte';
  import { shortcutsIpc, type ShortcutInfo } from '../../lib/ipc';

  const labels: Record<ShortcutInfo['id'], string> = {
    quick_download: 'Quick download from the clipboard',
    open_app: 'Open NOMNOM',
  };

  let items = $state<ShortcutInfo[]>([]);
  let recording = $state<ShortcutInfo['id'] | null>(null);
  let hint = $state('');
  let problem = $state<Record<string, string>>({});

  async function load() {
    items = await shortcutsIpc.status();
  }
  void load();

  async function onKey(e: KeyboardEvent, id: ShortcutInfo['id']) {
    if (recording !== id) return;
    e.preventDefault();
    e.stopPropagation();
    if (e.key === 'Escape') {
      recording = null;
      hint = '';
      return;
    }
    const { accelerator, needsModifier } = toAccelerator(e);
    if (needsModifier) hint = 'Hold Ctrl, Alt, Shift or Win while you press the key.';
    if (!accelerator) return;
    recording = null;
    hint = '';
    try {
      await shortcutsIpc.set(id, accelerator);
      problem = { ...problem, [id]: '' };
    } catch (err) {
      problem = { ...problem, [id]: String(err) };
    }
    await load();
  }
</script>

<section class="group">
  <h3>Shortcuts</h3>
  {#each items as item (item.id)}
    <div class="row">
      <div>
        <div class="label">{labels[item.id]}</div>
        {#if problem[item.id]}
          <div class="hint bad" role="alert">{problem[item.id]}</div>
        {:else if item.error}
          <div class="hint bad" role="alert">{item.error}</div>
        {/if}
      </div>
      <button
        class="key"
        class:recording={recording === item.id}
        aria-label={`${labels[item.id]}: ${item.accelerator}. Press to change.`}
        onclick={() => { recording = item.id; hint = ''; }}
        onkeydown={(e) => onKey(e, item.id)}
        onblur={() => { if (recording === item.id) { recording = null; hint = ''; } }}
      >
        {recording === item.id ? 'Press the new keys…' : item.accelerator}
      </button>
    </div>
  {/each}
  <div class="row">
    <div>
      <div class="label">Keep quick download ready</div>
      <div class="hint">Opens instantly. Without it, the window takes about a second to start. Uses about 200 MB of memory while NOMNOM is idle.</div>
    </div>
    <button
      class="switch"
      role="switch"
      aria-label="Keep quick download ready"
      aria-checked={app.settings?.shortcuts.keep_ready ?? false}
      class:on={app.settings?.shortcuts.keep_ready}
      onclick={() => app.update({ quickKeepReady: !app.settings?.shortcuts.keep_ready })}><span></span></button>
  </div>
  {#if hint}<p class="hint" role="status">{hint}</p>{/if}
  <p class="hint">Press Esc while recording to cancel.</p>
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; align-items: center; justify-content: space-between; gap: 24px; }
  .row + .row { margin-top: 14px; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .bad { color: var(--danger); }
  .key {
    min-width: 132px; padding: 6px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); cursor: pointer; font-variant-numeric: tabular-nums;
  }
  .key:hover { background: var(--surface-3); }
  .key.recording { border-color: var(--accent); color: var(--accent); }
  .switch { flex: none; width: 38px; height: 22px; border-radius: 11px; border: 0; padding: 2px; background: var(--surface-3); cursor: pointer; transition: background var(--dur-2); }
  .switch span { display: block; width: 18px; height: 18px; border-radius: 50%; background: var(--text-2); transition: transform var(--dur-2) var(--ease-spring), background var(--dur-2); }
  .switch.on { background: var(--accent); }
  .switch.on span { transform: translateX(16px); background: var(--on-accent); }
</style>
