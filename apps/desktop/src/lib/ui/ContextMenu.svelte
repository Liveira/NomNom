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
  let measured = $state(0);

  const left = $derived(Math.max(4, Math.min(x, window.innerWidth - 240)));
  const top = $derived(Math.max(4, Math.min(y, window.innerHeight - (measured || items.length * 34 + 16) - 4)));

  $effect(() => {
    const previous = document.activeElement as HTMLElement | null;
    el?.focus();
    const away = (e: MouseEvent) => {
      if (el && !el.contains(e.target as Node)) onclose();
    };
    window.addEventListener('mousedown', away, true);
    window.addEventListener('blur', onclose);
    return () => {
      window.removeEventListener('mousedown', away, true);
      window.removeEventListener('blur', onclose);
      const now = document.activeElement;
      if (previous && previous.isConnected && (now === document.body || now === null || el?.contains(now))) previous.focus();
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
      el?.querySelectorAll('button')[next]?.scrollIntoView({ block: 'nearest' });
    } else if (e.key === 'Enter' && active >= 0) {
      e.preventDefault();
      onpick(items[active].id);
    }
  }
</script>

<div class="menu" bind:this={el} bind:offsetHeight={measured} role="menu" tabindex="-1" style:left="{left}px" style:top="{top}px" {onkeydown}>
  {#each items as it, i (it.id)}
    {#if it.separatorBefore}<div class="sep" role="separator"></div>{/if}
    <button role="menuitem" class:active={i === active} disabled={it.disabled} onmouseenter={() => (active = i)} onclick={() => onpick(it.id)}>
      <span>{it.label}</span>
      {#if it.hint}<span class="hint">{it.hint}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .menu {
    position: fixed; z-index: 60; min-width: 216px; background: var(--surface-1);
    border: 1px solid var(--border-strong); border-radius: var(--radius-s); padding: 4px;
    box-shadow: var(--shadow-pop); outline: none; display: grid; max-height: calc(100vh - 8px); overflow-y: auto;
  }
  button { display: flex; justify-content: space-between; gap: 20px; border: 0; background: none; color: inherit; text-align: left; padding: 7px 10px; border-radius: var(--radius-xs); cursor: pointer; font-size: 13px; }
  button.active:not(:disabled), button:hover:not(:disabled) { background: var(--surface-2); color: var(--text); }
  button:disabled { color: var(--text-3); cursor: default; }
  .hint { color: var(--text-3); font-size: 12px; }
  .sep { height: 1px; background: var(--border); margin: 4px 2px; }
</style>
