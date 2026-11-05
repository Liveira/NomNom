<script lang="ts">
  let { size = 28 }: { size?: number } = $props();
</script>

<!-- A round-eared cat with its eyes closed, mid-bite. Colours come from the theme. -->
<svg width={size} height={size} viewBox="0 0 32 32" aria-hidden="true">
  <path d="M5.2 12.5 6 4.2 13 8.4Z M26.8 12.5 26 4.2 19 8.4Z" fill="var(--accent)" stroke="var(--accent)" stroke-width="2" stroke-linejoin="round" />
  <path d="M7.6 9.2 7.9 6.9 10.3 8.4Z M24.4 9.2 24.1 6.9 21.7 8.4Z" fill="var(--blush)" />
  <rect x="3" y="7.5" width="26" height="21" rx="10" fill="var(--accent)" />
  <ellipse cx="8.4" cy="20.6" rx="2.6" ry="1.7" fill="var(--blush)" opacity="0.55" />
  <ellipse cx="23.6" cy="20.6" rx="2.6" ry="1.7" fill="var(--blush)" opacity="0.55" />
  <path d="M9.6 17.4c.9-1.7 2.9-1.7 3.8 0M18.6 17.4c.9-1.7 2.9-1.7 3.8 0" stroke="var(--on-accent)" stroke-width="1.8" stroke-linecap="round" fill="none" />
  <ellipse cx="16" cy="20" rx="1.3" ry="1" fill="var(--blush)" />
  <path d="M13.2 21.6c1.2 1.7 2.1 1.1 2.8 0 .7 1.1 1.6 1.7 2.8 0" stroke="var(--on-accent)" stroke-width="1.4" stroke-linecap="round" stroke-linejoin="round" fill="none" />
</svg>
