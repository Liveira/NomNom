<script lang="ts">
  import Icon from '../icons/Icon.svelte';

  /** Minimal icon badge for page headers (kept API-compatible with `orbit` and `size`). */
  let {
    icon,
    orbit: _orbit = [],
    size = 48,
  }: { icon: string; orbit?: string[]; size?: number } = $props();

  const box = $derived(Math.min(52, Math.max(36, Math.round(size * 0.48))));
</script>

<div class="emblem" style:width="{box}px" style:height="{box}px" aria-hidden="true">
  <Icon name={icon} size={Math.round(box * 0.5)} stroke={1.9} />
</div>

<style>
  .emblem {
    flex: none;
    display: grid;
    place-items: center;
    border-radius: var(--radius-m);
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--accent-text);
  }
</style>
