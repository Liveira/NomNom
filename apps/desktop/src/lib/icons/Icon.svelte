<script lang="ts">
  import { ICONS } from './registry';

  /** Lucide icons (ISC licence). `name` is one of the keys in registry.ts. */
  let { name, size = 20, stroke = 1.9 }: { name: string; size?: number; stroke?: number } = $props();
  const Glyph = $derived(ICONS[name] ?? ICONS.home);
</script>

<Glyph {size} strokeWidth={stroke} aria-hidden="true" />
