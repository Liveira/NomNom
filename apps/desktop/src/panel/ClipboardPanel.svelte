<script lang="ts">
  import ClipList from '../modules/clipboard/ClipList.svelte';
</script>

<ClipList mode="popup" />

<style>
  :global(html), :global(body) { background: var(--surface-1); overflow: hidden; }
</style>
