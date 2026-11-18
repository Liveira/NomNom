<script lang="ts">
  import type { Snippet } from 'svelte';

  /** A modal window. Escape closes it (unless `locked`), and focus stays inside while it is open. */
  let {
    title,
    onclose,
    locked = false,
    children,
    actions,
  }: { title: string; onclose: () => void; locked?: boolean; children: Snippet; actions?: Snippet } = $props();

  let box = $state<HTMLDivElement | null>(null);
  let previous: Element | null = null;

  $effect(() => {
    previous = document.activeElement;
    const first = box?.querySelector<HTMLElement>('input, textarea, button.primary, button');
    first?.focus();
    return () => (previous as HTMLElement | null)?.focus?.();
  });

  function onkeydown(e: KeyboardEvent) {
    e.stopPropagation();
    if (e.key === 'Escape' && !locked) {
      e.preventDefault();
      onclose();
    } else if (e.key === 'Tab' && box) {
      const f = [...box.querySelectorAll<HTMLElement>('button, input, select, textarea, [tabindex="0"]')].filter((x) => !(x as HTMLButtonElement).disabled);
      if (f.length === 0) return;
      const i = f.indexOf(document.activeElement as HTMLElement);
      if (e.shiftKey && i <= 0) {
        e.preventDefault();
        f[f.length - 1].focus();
      } else if (!e.shiftKey && i === f.length - 1) {
        e.preventDefault();
        f[0].focus();
      }
    }
  }
</script>

<div class="scrim" role="presentation" onclick={() => !locked && onclose()}>
  <div class="box" bind:this={box} role="dialog" aria-modal="true" aria-label={title} tabindex="-1" onclick={(e) => e.stopPropagation()} {onkeydown}>
    <h2>{title}</h2>
    <div class="content">{@render children()}</div>
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
</div>

<style>
  .scrim {
    position: fixed;
    inset: 0;
    background: rgba(9, 6, 15, 0.72);
    backdrop-filter: blur(6px);
    display: grid;
    place-items: center;
    z-index: 50;
    animation: fade var(--dur-2) var(--ease-out);
  }
  .box {
    width: min(600px, calc(100vw - 48px));
    max-height: calc(100vh - 72px);
    display: grid;
    grid-template-rows: auto 1fr auto;
    gap: 14px;
    background: linear-gradient(165deg, var(--surface-2) 0%, var(--surface-1) 100%);
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-l);
    padding: 20px 24px;
    box-shadow: var(--shadow-pop);
    animation: pop var(--dur-2) var(--ease-spring);
  }
  h2 { font-size: 18px; }
  .content { overflow: auto; min-height: 0; }
  .actions { display: flex; justify-content: flex-end; gap: 8px; padding-top: 4px; }
  @keyframes fade { from { opacity: 0; } to { opacity: 1; } }
  @keyframes pop { from { opacity: 0; transform: translateY(8px) scale(0.97); } to { opacity: 1; transform: none; } }
</style>
