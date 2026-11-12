<script lang="ts" module>
  export interface MenuItem {
    id: string;
    label: string;
    hint?: string;
    disabled?: boolean;
    separatorBefore?: boolean;
  }
</script>

<script lang="ts">
  let { x, y, items, onpick, onclose }: { x: number; y: number; items: MenuItem[]; onpick: (id: string) => void; onclose: () => void } = $props();

  let el = $state<HTMLDivElement | null>(null);
  let active = $state(-1);

  // Keep the menu inside the window.
  const left = $derived(Math.max(4, Math.min(x, window.innerWidth - 240)));
  const top = $derived(Math.max(4, Math.min(y, window.innerHeight - (items.length * 30 + 16))));

  $effect(() => {
    el?.focus();
    const away = (e: MouseEvent) => {
      if (el && !el.contains(e.target as Node)) onclose();
    };
    window.addEventListener('mousedown', away, true);
    window.addEventListener('blur', onclose);
    return () => {
      window.removeEventListener('mousedown', away, true);
      window.removeEventListener('blur', onclose);
    };
  });

  function onkeydown(e: KeyboardEvent) {
    const enabled = items.map((it, i) => (it.disabled ? -1 : i)).filter((i) => i >= 0);
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onclose();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowUp') {
      e.preventDefault();
      if (enabled.length === 0) return;
      const at = enabled.indexOf(active);
      const next = e.key === 'ArrowDown' ? enabled[(at + 1) % enabled.length] : enabled[(at <= 0 ? enabled.length : at) - 1];
      active = next;
    } else if (e.key === 'Enter' && active >= 0) {
      e.preventDefault();
      onpick(items[active].id);
    }
  }
</script>

<div class="menu" bind:this={el} role="menu" tabindex="-1" style:left="{left}px" style:top="{top}px" {onkeydown}>
  {#each items as it, i (it.id)}
    {#if it.separatorBefore}<div class="sep" role="separator"></div>{/if}
    <button role="menuitem" class:active={i === active} disabled={it.disabled} onmouseenter={() => (active = i)} onclick={() => onpick(it.id)}>
      <span>{it.label}</span>
      {#if it.hint}<span class="hint">{it.hint}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .menu { position: fixed; z-index: 60; min-width: 220px; background: var(--surface-2); border: 1px solid var(--border-strong); border-radius: var(--radius-s); padding: 4px; box-shadow: 0 10px 30px rgba(0, 0, 0, 0.4); outline: none; display: grid; }
  button { display: flex; justify-content: space-between; gap: 24px; border: 0; background: none; color: inherit; text-align: left; padding: 6px 10px; border-radius: 6px; cursor: pointer; }
  button.active:not(:disabled), button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { color: var(--text-3); cursor: default; }
  .hint { color: var(--text-3); font-size: 12px; }
  .sep { height: 1px; background: var(--border); margin: 4px 2px; }
</style>
