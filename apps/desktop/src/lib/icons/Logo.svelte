<script lang="ts">
  import Icon from './Icon.svelte';

  let { size = 28 }: { size?: number } = $props();
</script>

<!-- NomNom's mark: clean rounded tile in the theme accent. -->
<span class="logo" style:width="{size}px" style:height="{size}px" style:border-radius="{Math.max(5, Math.round(size * 0.28))}px" aria-hidden="true">
  <Icon name="cat" size={Math.round(size * 0.62)} stroke={2.1} />
</span>

<style>
  .logo {
    display: inline-grid;
    place-items: center;
    flex: none;
    color: var(--on-accent);
    background: var(--accent);
  }
</style>
