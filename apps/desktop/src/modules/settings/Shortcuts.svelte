<script lang="ts">
  import { isSuperKey, toAccelerator } from '../../lib/accelerator';
  import { app } from '../../app/state.svelte';
  import { isLinux } from '../../lib/platform';
  import { shortcutsIpc, type ShortcutInfo } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';

  const labels: Record<ShortcutInfo['id'], string> = {
    quick_download: 'Quick download from the clipboard',
    open_app: 'Open NomNom',
    clipboard: 'Open the clipboard history',
    translate_replace: 'Replace the selected text with its translation',
    translate_popup: 'Show the translation of the selected text',
    capture_text: 'Read text from the screen',
    launcher: 'Open the launcher (search programs, files and your actions)',
    notes: 'Write a quick note (when Notes is in the sidebar)',
    pin_top: 'Keep the window in front on top, or let it go',
    pick_color: 'Pick a colour from the screen',
    paste_plain: 'Paste as plain text',
    clean_link: 'Clean the link on the clipboard',
    screenshot: 'Take a screenshot to mark up',
    record: 'Record an area of the screen, or stop recording',
  };

  let items = $state<ShortcutInfo[]>([]);
  let recording = $state<ShortcutInfo['id'] | null>(null);
  let hint = $state('');
  let problem = $state<Record<string, string>>({});
  /** Held down right now. Read from the key itself, because `metaKey` isn't reliable on Linux. */
  let superHeld = false;

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
    if (isSuperKey(e)) {
      superHeld = true;
      return;
    }
    const { accelerator, needsModifier } = toAccelerator({
      code: e.code,
      ctrlKey: e.ctrlKey,
      altKey: e.altKey,
      shiftKey: e.shiftKey,
      metaKey: e.metaKey || superHeld,
    });
    if (needsModifier) hint = `Hold Ctrl, Alt, Shift or ${isLinux ? 'Super' : 'Win'} while you press the key.`;
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
        onkeyup={(e) => { if (isSuperKey(e)) superHeld = false; }}
        onblur={() => { superHeld = false; if (recording === item.id) { recording = null; hint = ''; } }}
      >
        {recording === item.id ? 'Press the new keys…' : item.accelerator}
      </button>
    </div>
  {/each}
  <div class="row">
    <div>
      <div class="label">Use the launcher shortcut</div>
      <div class="hint">Turn it off if another app, such as PowerToys, already uses these keys.</div>
    </div>
    <Switch
      checked={app.settings?.shortcuts.launcher_on ?? true}
      label="Use the launcher shortcut"
      onchange={(v) => app.update({ launcherOn: v })}
    />
  </div>
  <div class="row">
    <div>
      <div class="label">Keep quick download ready</div>
      <div class="hint">Opens instantly. Without it, the window takes about a second to start. Uses about 200 MB of memory while NomNom is idle.</div>
    </div>
    <Switch
      checked={app.settings?.shortcuts.keep_ready ?? false}
      label="Keep quick download ready"
      onchange={(v) => app.update({ quickKeepReady: v })}
    />
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
  .key.recording { border-color: var(--accent); color: var(--accent-text); }
</style>
