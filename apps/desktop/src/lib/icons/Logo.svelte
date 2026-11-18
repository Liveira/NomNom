<script lang="ts">
  import Icon from './Icon.svelte';

  let { size = 30 }: { size?: number } = $props();
</script>

<!-- NomNom's mark: the Lucide "cat" icon (ISC licence) on a gradient tile, the same as the app icon. -->
<span class="logo" style:width="{size}px" style:height="{size}px" style:border-radius="{Math.round(size * 0.3)}px" aria-hidden="true">
  <Icon name="cat" size={Math.round(size * 0.62)} stroke={2.1} />
</span>

<style>
  .logo {
    display: inline-grid;
    place-items: center;
    flex: none;
    color: var(--on-accent);
    background: linear-gradient(145deg, var(--accent), var(--blush));
    box-shadow: 0 4px 14px var(--glow-peach), inset 0 1px 0 rgba(255, 255, 255, 0.35);
    transition: transform var(--dur-2) var(--ease-spring);
  }
  .logo:hover { transform: rotate(-8deg) scale(1.08); }
</style>
