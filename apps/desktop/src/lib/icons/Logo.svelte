<script lang="ts">
  let { size = 30 }: { size?: number } = $props();
</script>

<!-- NomNom mascot emblem: plush round-eared mochi cat with inner sakura ears, happy eyes, and a tiny fang. -->
<svg width={size} height={size} viewBox="0 0 36 36" aria-hidden="true">
  <path
    d="M5.6 14.2 6.8 4.4 15.2 9.4Z M30.4 14.2 29.2 4.4 20.8 9.4Z"
    fill="var(--accent)"
    stroke="var(--accent)"
    stroke-width="2.2"
    stroke-linejoin="round"
  />
  <path d="M8.3 10.6 8.9 7.2 12.1 9.2Z M27.7 10.6 27.1 7.2 23.9 9.2Z" fill="var(--blush)" />
  <rect x="3.5" y="8.8" width="29" height="23" rx="11.5" fill="var(--accent)" />
  <path
    d="M8.5 11.4c4.5-1.5 10.5-1.5 15 0"
    stroke="rgba(255,255,255,0.42)"
    stroke-width="1.6"
    stroke-linecap="round"
    fill="none"
  />
  <ellipse cx="9.2" cy="23.2" rx="3" ry="1.9" fill="var(--blush)" opacity="0.65" />
  <ellipse cx="26.8" cy="23.2" rx="3" ry="1.9" fill="var(--blush)" opacity="0.65" />
  <path
    d="M10.4 19.3c1.1-2 3.4-2 4.5 0M21.1 19.3c1.1-2 3.4-2 4.5 0"
    stroke="var(--on-accent)"
    stroke-width="2.1"
    stroke-linecap="round"
    fill="none"
  />
  <ellipse cx="18" cy="22" rx="1.5" ry="1.15" fill="var(--blush)" />
  <path
    d="M14.7 23.9c1.3 1.9 2.4 1.2 3.3 0 .9 1.2 2 1.9 3.3 0"
    stroke="var(--on-accent)"
    stroke-width="1.6"
    stroke-linecap="round"
    stroke-linejoin="round"
    fill="none"
  />
  <path d="M20.2 24.7 21 26.5 21.8 24.6Z" fill="#fffaf5" />
</svg>
