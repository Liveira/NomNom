<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { trIpc, type PopupData } from '../lib/ipc';
  import { fromLabel } from '../modules/text/logic';

  let data = $state<PopupData | null>(null);
  let shown = false;
  const loading = $derived(!!data && data.translated === null && data.error === null);

  async function refresh() {
    data = await trIpc.popupTake();
    if (data && !shown) {
      shown = true;
      await trIpc.popupReady();
    }
  }

  onMount(() => {
    void refresh();
    const off = listen('translate://done', () => void refresh());
    // The answer can arrive before this page listens, so it also asks now and then while waiting.
    const poll = setInterval(() => loading && void refresh(), 500);
    return () => {
      clearInterval(poll);
      void off.then((f) => f());
    };
  });

  function onKey(e: KeyboardEvent) {
    if (e.key === 'Escape') void trIpc.popupClose();
  }
</script>

<svelte:window onkeydown={onKey} />

{#if data}
  <div class="card" role="dialog" aria-label="Translation">
    <header>
      <Icon name="languages" size={16} />
      <span class="title">Translation</span>
      {#if data.detected}<span class="langs" translate="no">{fromLabel(data.detected)} → {data.target}</span>{/if}
      <button class="x" aria-label="Close" onclick={() => trIpc.popupClose()}><Icon name="x" size={14} /></button>
    </header>
    <div class="orig" translate="no">{data.original}</div>
    <div class="out" class:loading aria-live="polite">
      {#if data.error}
        <span class="err">{data.error}</span>
      {:else if loading}
        <span class="dots">Translating…</span>
      {:else}
        <span translate="no">{data.translated}</span>
      {/if}
    </div>
    <footer>
      <button class="plain" disabled={!data.translated} onclick={() => trIpc.popupUse('copy')}><Icon name="copy" size={14} /> Copy</button>
      <button class="primary" disabled={!data.translated} onclick={() => trIpc.popupUse('replace')}>Use in place</button>
    </footer>
  </div>
{/if}

<style>
  :global(html), :global(body) { background: var(--surface-1); overflow: hidden; }
  .card { height: 100vh; box-sizing: border-box; display: grid; grid-template-rows: auto auto 1fr auto; gap: 10px; padding: 12px 14px; background: var(--surface-1); border: 1px solid var(--border-strong); }
  header { display: flex; align-items: center; gap: 8px; color: var(--accent-text); }
  .title { font-family: var(--font-display); font-weight: 700; color: var(--text); font-size: 14px; }
  .langs { margin-left: auto; font-size: 11.5px; color: var(--text-3); font-family: var(--font-mono); }
  .x { border: 0; background: none; color: var(--text-3); cursor: pointer; padding: 3px; border-radius: 8px; display: grid; }
  .x:hover { background: var(--surface-3); color: var(--text); }
  .orig { color: var(--text-3); font-size: 12.5px; max-height: 56px; overflow: auto; white-space: pre-wrap; word-break: break-word; }
  .out { overflow: auto; font-size: 15px; line-height: 1.45; white-space: pre-wrap; word-break: break-word; padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .out.loading { color: var(--text-3); }
  .err { color: var(--danger); }
  footer { display: flex; justify-content: flex-end; gap: 8px; }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button:disabled { opacity: 0.5; cursor: default; }
</style>
