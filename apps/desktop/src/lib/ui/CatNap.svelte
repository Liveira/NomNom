<script lang="ts">
  let { width = 112 }: { width?: number } = $props();
</script>

<!-- A cat asleep next to the empty list. Decorative, so hidden from screen readers. -->
<svg {width} height={width * 0.6} viewBox="0 0 112 67" aria-hidden="true" class="nap">
  <g fill="var(--surface-3)">
    <ellipse cx="62" cy="49" rx="42" ry="16" />
    <circle cx="26" cy="45" r="14" />
    <path d="M14.5 36 15.5 24 25 31Z M37.5 36 36.5 24 27 31Z" stroke="var(--surface-3)" stroke-width="3" stroke-linejoin="round" />
  </g>
  <path d="M101 52q11-3 6-16" stroke="var(--surface-3)" stroke-width="7" stroke-linecap="round" fill="none" />
  <path d="M19.5 45q2.5 2.6 5 0M28 45q2.5 2.6 5 0" stroke="var(--text-3)" stroke-width="1.6" stroke-linecap="round" fill="none" />
  <ellipse cx="26" cy="49.2" rx="1.5" ry="1.1" fill="var(--blush)" />
  <g fill="var(--text-3)" font-family="var(--font-display)" font-weight="600">
    <text x="46" y="26" font-size="11">z</text>
    <text x="56" y="16" font-size="8">z</text>
  </g>
</svg>

<style>
  .nap { display: block; }
</style>
