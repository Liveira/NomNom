<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  /** Draws only the rows that are visible, so a folder with 100 000 entries scrolls smoothly. */
  let {
    items,
    rowHeight = 28,
    row,
    label = 'List',
    onkeydown,
  }: {
    items: T[];
    rowHeight?: number;
    row: Snippet<[T, number]>;
    label?: string;
    onkeydown?: (e: KeyboardEvent) => void;
  } = $props();

  let top = $state(0);
  let height = $state(400);
  let el = $state<HTMLDivElement | null>(null);

  const overscan = 6;
  const first = $derived(Math.max(0, Math.floor(top / rowHeight) - overscan));
  const last = $derived(Math.min(items.length, Math.ceil((top + height) / rowHeight) + overscan));
  const visible = $derived(items.slice(first, last));

  /** Scrolls so that row `index` is fully inside the view. */
  export function reveal(index: number) {
    if (!el) return;
    const y = index * rowHeight;
    if (y < el.scrollTop) el.scrollTop = y;
    else if (y + rowHeight > el.scrollTop + el.clientHeight) el.scrollTop = y + rowHeight - el.clientHeight;
  }
  export function focus() {
    el?.focus();
  }
</script>

<div
  class="vl"
  bind:this={el}
  bind:clientHeight={height}
  onscroll={(e) => (top = e.currentTarget.scrollTop)}
  role="listbox"
  aria-label={label}
  aria-multiselectable="true"
  tabindex="0"
  {onkeydown}
>
  <div class="spacer" style:height="{items.length * rowHeight}px">
    <div class="window" style:transform="translateY({first * rowHeight}px)">
      {#each visible as item, i (first + i)}
        <div class="row" style:height="{rowHeight}px">
          {@render row(item, first + i)}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .vl { overflow: auto; height: 100%; min-height: 0; outline: none; position: relative; }
  .spacer { position: relative; min-width: 100%; }
  .window { position: absolute; left: 0; right: 0; top: 0; will-change: transform; }
  .row { display: block; }
</style>
