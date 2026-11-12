<script lang="ts" generics="T">
  import type { Snippet } from 'svelte';

  /** Draws only the rows that are visible, so a folder with 100 000 entries scrolls smoothly. */
  let {
    items,
    rowHeight = 28,
    row,
    label = 'List',
    kind = 'listbox',
    onkeydown,
  }: {
    items: T[];
    rowHeight?: number;
    row: Snippet<[T, number]>;
    label?: string;
    /** 'listbox' when rows can be selected, 'list' for rows that only show information and hold their own buttons. */
    kind?: 'listbox' | 'list';
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

<!-- A list that scrolls has to take focus, or a keyboard user can't scroll it. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div
  class="vl"
  bind:this={el}
  bind:clientHeight={height}
  onscroll={(e) => (top = e.currentTarget.scrollTop)}
  role={kind}
  aria-label={label}
  aria-multiselectable={kind === 'listbox' ? 'true' : undefined}
  tabindex="0"
  {onkeydown}
>
  <div class="spacer" style:height="{items.length * rowHeight}px">
    <div class="window" style:transform="translateY({first * rowHeight}px)">
      {#each visible as item, i (first + i)}
        <div class="row" role={kind === 'list' ? 'listitem' : undefined} style:height="{rowHeight}px">
          {@render row(item, first + i)}
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
  .vl { overflow: auto; height: 100%; min-height: 0; outline: none; position: relative; }
  /* Drawn inside the box: the list clips anything outside it. */
  .vl:focus-visible { outline: 2px solid var(--focus); outline-offset: -2px; }
  .spacer { position: relative; min-width: 100%; }
  .window { position: absolute; left: 0; right: 0; top: 0; will-change: transform; }
  .row { display: block; }
</style>
