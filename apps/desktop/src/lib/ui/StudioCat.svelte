<script lang="ts">
  /** Artistic cat mascot vignette for section headers. Purely decorative (`aria-hidden`). */
  let {
    mood = 'studio',
    size = 86,
  }: { mood?: 'studio' | 'gamer' | 'dj' | 'alchemist' | 'keeper'; size?: number } = $props();
</script>

<svg width={size} height={Math.round(size * 0.84)} viewBox="0 0 108 90" aria-hidden="true" class="mascot">
  <!-- Soft ambient halo -->
  <ellipse cx="56" cy="76" rx="40" ry="9" fill="var(--surface-3)" opacity="0.65" />
  <circle cx="88" cy="20" r="2.2" fill="var(--lavender)" opacity="0.85" />
  <path d="M18 18l1.3 3.4 3.4 1.3-3.4 1.3-1.3 3.4-1.3-3.4-3.4-1.3 3.4-1.3 1.3-3.4Z" fill="var(--accent)" opacity="0.9" />

  <!-- Tail -->
  <path d="M80 66c12-1 17-10 12-20" stroke="var(--accent)" stroke-width="6.5" stroke-linecap="round" fill="none" />

  <!-- Body -->
  <rect x="33" y="48" width="44" height="30" rx="15" fill="var(--surface-3)" stroke="var(--border-strong)" stroke-width="1.2" />

  <!-- Head & ears -->
  <path
    d="M31 32 33 13 48 23Z M79 32 77 13 62 23Z"
    fill="var(--accent)"
    stroke="var(--accent)"
    stroke-width="2.2"
    stroke-linejoin="round"
  />
  <path d="M35.5 25.5 36.4 18.8 42.6 22.8Z M74.5 25.5 73.6 18.8 67.4 22.8Z" fill="var(--blush)" />
  <rect x="27" y="21" width="56" height="40" rx="19" fill="var(--accent)" />

  {#if mood === 'dj'}
    <!-- Cozy studio headphones -->
    <path d="M25 38a30 27 0 0 1 60 0" stroke="var(--lavender)" stroke-width="4.2" fill="none" stroke-linecap="round" />
    <rect x="20" y="33" width="8.5" height="16" rx="4.2" fill="var(--lavender)" />
    <rect x="81.5" y="33" width="8.5" height="16" rx="4.2" fill="var(--lavender)" />
  {:else if mood === 'gamer'}
    <!-- Cute visor / headset mic + controller -->
    <path d="M26 37a29 25 0 0 1 58 0" stroke="var(--blush)" stroke-width="3.8" fill="none" stroke-linecap="round" />
    <rect x="21" y="34" width="7.5" height="14" rx="3.7" fill="var(--blush)" />
    <rect x="81.5" y="34" width="7.5" height="14" rx="3.7" fill="var(--blush)" />
  {:else if mood === 'alchemist'}
    <!-- Little beret / artist cap -->
    <ellipse cx="55" cy="20" rx="18" ry="6.5" fill="var(--lavender)" transform="rotate(-7 55 20)" />
    <circle cx="54" cy="13.5" r="2.5" fill="var(--blush)" />
  {/if}

  <!-- Cheeks, eyes & mouth -->
  <ellipse cx="37" cy="45.5" rx="4.2" ry="2.6" fill="var(--blush)" opacity="0.65" />
  <ellipse cx="73" cy="45.5" rx="4.2" ry="2.6" fill="var(--blush)" opacity="0.65" />
  <circle cx="43" cy="39.5" r="3.1" fill="var(--on-accent)" />
  <circle cx="67" cy="39.5" r="3.1" fill="var(--on-accent)" />
  <circle cx="44" cy="38.4" r="1.1" fill="#ffffff" />
  <circle cx="68" cy="38.4" r="1.1" fill="#ffffff" />
  <ellipse cx="55" cy="43.2" rx="2" ry="1.4" fill="var(--blush)" />
  <path
    d="M50.6 46c1.6 2.1 3.1 1.4 4.4 0 1.3 1.4 2.8 2.1 4.4 0"
    stroke="var(--on-accent)"
    stroke-width="1.8"
    stroke-linecap="round"
    stroke-linejoin="round"
    fill="none"
  />

  <!-- Foreground prop by mood -->
  {#if mood === 'gamer'}
    <!-- Mini gamepad held in paws -->
    <rect x="41" y="59" width="28" height="15" rx="7.5" fill="var(--surface-1)" stroke="var(--lavender)" stroke-width="1.6" />
    <path d="M48 66.5h5M50.5 64v5" stroke="var(--ok)" stroke-width="1.8" stroke-linecap="round" />
    <circle cx="61" cy="65" r="1.6" fill="var(--blush)" />
    <circle cx="64.2" cy="68" r="1.6" fill="var(--accent)" />
  {:else if mood === 'studio'}
    <!-- Drawing stylus & glowing tablet -->
    <rect x="39" y="61" width="32" height="13" rx="4.5" fill="var(--surface-1)" stroke="var(--info)" stroke-width="1.5" />
    <path d="M62 67 76 51" stroke="var(--blush)" stroke-width="3.2" stroke-linecap="round" />
    <circle cx="62" cy="67" r="2" fill="var(--ok)" />
  {:else if mood === 'keeper'}
    <!-- Tidy folder parcel -->
    <rect x="40" y="58" width="30" height="17" rx="4" fill="var(--ok)" opacity="0.9" />
    <path d="M40 64h30M55 58v17" stroke="var(--on-accent)" stroke-width="1.6" />
  {:else if mood === 'alchemist'}
    <!-- Glowing crystal prism -->
    <polygon points="55,55 65,67 55,76 45,67" fill="var(--info)" stroke="#ffffff" stroke-width="1.2" opacity="0.92" />
  {:else}
    <!-- Paws -->
    <ellipse cx="45" cy="68" rx="5.5" ry="4" fill="var(--accent)" />
    <ellipse cx="65" cy="68" rx="5.5" ry="4" fill="var(--accent)" />
  {/if}
</svg>

<style>
  .mascot {
    display: block;
    flex: none;
    filter: drop-shadow(0 6px 14px rgba(6, 3, 14, 0.35));
    transition: transform var(--dur-3) var(--ease-spring);
  }
  .mascot:hover {
    transform: translateY(-2px) rotate(-1.5deg);
  }
</style>
