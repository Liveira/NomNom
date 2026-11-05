<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<!-- A rounded square with a bite taken out of the top-right corner, and a crumb. -->
<svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  <defs>
    <mask id="bite">
      <rect width="32" height="32" fill="white" />
      <circle cx="27" cy="5" r="7" fill="black" />
    </mask>
  </defs>
  <rect x="2" y="2" width="28" height="28" rx="9" fill="var(--accent)" mask="url(#bite)" />
  <circle cx="24.5" cy="12.5" r="1.6" fill="var(--accent)" />
  <path
    d="M9.5 18.5c1.6 2.6 4.4 3.5 6.8 2.5"
    stroke="var(--on-accent)"
    stroke-width="2.2"
    stroke-linecap="round"
    fill="none"
  />
</svg>
