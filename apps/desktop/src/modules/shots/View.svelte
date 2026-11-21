<script lang="ts">
  import { onMount } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { shortcutsIpc, shotIpc, type ShortcutInfo } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';

  let key = $state<ShortcutInfo | null>(null);
  onMount(async () => {
    key = (await shortcutsIpc.status()).find((s) => s.id === 'screenshot') ?? null;
  });
</script>

<header class="head">
  <h1><Icon name="image" size={24} /> Screenshots</h1>
  <p class="sub">Take a picture of part of the screen and mark it up before you send it: arrows, boxes, a marker, numbered steps, text, and blocked-out parts for what must not be seen.</p>
</header>

{#if isLinux}
  <p class="hint">Screenshots with notes work on Windows for now. On Linux, your desktop's own screenshot tool does this well.</p>
{:else}
  <section class="card">
    <ol>
      <li>Press <kbd>{key?.accelerator ?? 'Ctrl+Alt+S'}</kbd> (or the button below). The screen freezes.</li>
      <li>Drag over the part you want.</li>
      <li>Add arrows, boxes and text. <strong>Hide</strong> turns a part into blocks, so a password or a name cannot be read back.</li>
      <li>Copy it to paste in a chat, or save it as a PNG.</li>
    </ol>
    {#if key?.error}<p class="bad" role="alert">{key.error}</p>{/if}
    <button class="primary" onclick={() => void shotIpc.start()}>Take a screenshot now</button>
    <p class="hint">The picture stays in memory until you save it yourself. NomNom never uploads it anywhere.</p>
  </section>
{/if}

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; max-width: 640px; }
  .bad { color: var(--danger); font-size: 13px; }
  .card { display: grid; gap: 12px; justify-items: start; max-width: 640px; padding: 16px 20px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border-strong); }
  ol { margin: 0; padding-left: 20px; display: grid; gap: 6px; color: var(--text-2); font-size: 14px; }
  kbd { font: 600 12px var(--font-mono); padding: 2px 8px; border-radius: 6px; border: 1px solid var(--border-strong); color: var(--text); }
  .primary { display: inline-flex; padding: 8px 16px; border-radius: var(--radius-s); border: 0; background: var(--accent); color: var(--on-accent); cursor: pointer; font: inherit; font-weight: 600; }
  .primary:hover:not(:disabled) { background: var(--accent-hover); }
  .primary:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
</style>
