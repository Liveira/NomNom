<script lang="ts">
  import { categoryInfo, monogram } from './logic';

  let {
    name,
    category = 'general',
    iconUrl = null,
    size = 40,
  }: {
    name: string;
    category?: string;
    iconUrl?: string | null;
    size?: number;
  } = $props();

  let broken = $state(false);
  const cat = $derived(categoryInfo(category));
</script>

<div
  class="badge"
  style:width="{size}px"
  style:height="{size}px"
  style:--cat-color={cat.color}
  style:--cat-tint={cat.tint}
  aria-hidden="true"
>
  {#if iconUrl && !broken}
    <img src={iconUrl} alt="" width={Math.round(size * 0.68)} height={Math.round(size * 0.68)} onerror={() => (broken = true)} />
  {:else}
    <span style:font-size="{Math.max(11, Math.round(size * 0.34))}px">{monogram(name)}</span>
  {/if}
</div>

<style>
  .badge {
    flex: none;
    display: grid;
    place-items: center;
    border-radius: 24%;
    background: var(--cat-tint);
    border: 1px solid var(--border);
    overflow: hidden;
  }
  img {
    display: block;
    object-fit: contain;
  }
  span {
    font-family: var(--font-display);
    font-weight: 700;
    color: var(--cat-color);
    letter-spacing: 0.02em;
  }
</style>
