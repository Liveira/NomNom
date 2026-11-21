<script lang="ts">
  let { width = 76 }: { width?: number } = $props();
  const w = $derived(Math.min(width, 80));
</script>

<!-- Minimal sleeping cat mark for empty states. Decorative, hidden from screen readers. -->
<svg width={w} height={Math.round(w * 0.62)} viewBox="0 0 88 54" aria-hidden="true" class="nap">
  <ellipse cx="46" cy="40" rx="28" ry="11" fill="var(--surface-2)" stroke="var(--border-strong)" stroke-width="1.2" />
  <circle cx="24" cy="36" r="10.5" fill="var(--surface-2)" stroke="var(--border-strong)" stroke-width="1.2" />
  <path d="M16 29 17 20.5 23.5 26M32 29 31 20.5 24.5 26" fill="var(--surface-2)" stroke="var(--border-strong)" stroke-width="1.2" stroke-linejoin="round" />
  <path d="M72 41q9-2 6-11" stroke="var(--border-strong)" stroke-width="4" stroke-linecap="round" fill="none" />
  <path d="M19.5 36.2q1.8 2 3.6 0M25.4 36.2q1.8 2 3.6 0" stroke="var(--text-3)" stroke-width="1.4" stroke-linecap="round" fill="none" />
  <circle cx="24.2" cy="39" r="1" fill="var(--accent)" />
  <text x="41" y="20" font-size="9.5" fill="var(--text-3)" font-family="var(--font-ui)" font-weight="700">z</text>
  <text x="49" y="13" font-size="7.5" fill="var(--text-3)" font-family="var(--font-ui)" font-weight="700">z</text>
</svg>

<style>
  .nap { display: block; flex: none; opacity: 0.9; }
</style>
