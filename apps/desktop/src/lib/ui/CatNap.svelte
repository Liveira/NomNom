<script lang="ts">
  let { width = 116 }: { width?: number } = $props();
</script>

<!-- Sleeping studio cat curled up on a soft cushion with floating stars. Decorative, hidden from screen readers. -->
<svg {width} height={Math.round(width * 0.64)} viewBox="0 0 124 80" aria-hidden="true" class="nap">
  <ellipse cx="64" cy="64" rx="50" ry="11" fill="var(--surface-2)" stroke="var(--border-strong)" stroke-width="1" />
  <g fill="var(--surface-3)">
    <ellipse cx="66" cy="54" rx="39" ry="16.5" />
    <circle cx="31" cy="49" r="15" />
    <path
      d="M18.5 39 19.8 25.5 29.5 33.5Z M43.5 39 42.2 25.5 32.5 33.5Z"
      stroke="var(--surface-3)"
      stroke-width="3"
      stroke-linejoin="round"
    />
  </g>
  <path d="M21.4 35.6 22 30.2 26.8 34Z M40.6 35.6 40 30.2 35.2 34Z" fill="var(--blush)" opacity="0.55" />
  <path d="M102 56q13-3 8-18" stroke="var(--surface-3)" stroke-width="7.5" stroke-linecap="round" fill="none" />
  <circle cx="110" cy="38" r="3.6" fill="var(--accent)" opacity="0.7" />
  <ellipse cx="23" cy="53" rx="2.5" ry="1.5" fill="var(--blush)" opacity="0.5" />
  <ellipse cx="39" cy="53" rx="2.5" ry="1.5" fill="var(--blush)" opacity="0.5" />
  <path d="M24 48.5q2.6 2.8 5.2 0M32.8 48.5q2.6 2.8 5.2 0" stroke="var(--text-2)" stroke-width="1.7" stroke-linecap="round" fill="none" />
  <ellipse cx="31" cy="52.6" rx="1.5" ry="1.1" fill="var(--blush)" />
  <g fill="var(--lavender)" font-family="var(--font-display)" font-weight="700">
    <text x="50" y="29" font-size="11.5">z</text>
    <text x="61" y="19" font-size="9">z</text>
  </g>
  <path d="M77 14l1.2 3 3 1.2-3 1.2-1.2 3-1.2-3-3-1.2 3-1.2 1.2-3Z" fill="var(--accent)" opacity="0.85" />
</svg>

<style>
  .nap { display: block; flex: none; }
</style>
