<script lang="ts">
  /**
   * Subtle ambient wash behind the window (and inside theme preview cards).
   * Intentionally quiet and minimalist: soft colour atmosphere without distracting particles,
   * scanlines, or visual noise behind text and controls.
   */
  let {
    theme,
    motion = 'full',
    contained = false,
  }: { theme: string; motion?: 'full' | 'calm' | 'off'; contained?: boolean } = $props();

  const live = $derived(motion === 'full');
</script>

<div class="backdrop" class:contained class:still={!live} data-theme={theme} aria-hidden="true">
  <i class="wash w1"></i>
  <i class="wash w2"></i>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
    contain: strict;
    opacity: 0.65;
  }
  .backdrop.contained {
    position: absolute;
    border-radius: inherit;
    opacity: 1;
  }
  .wash {
    position: absolute;
    display: block;
    border-radius: 50%;
    will-change: transform;
    -webkit-mask-image: radial-gradient(closest-side, #000 10%, transparent 100%);
    mask-image: radial-gradient(closest-side, #000 10%, transparent 100%);
  }
  .w1 {
    width: 60%;
    height: 60%;
    right: -14%;
    top: -22%;
    background: var(--glow-peach);
    animation: drift-a 28s ease-in-out infinite alternate;
  }
  .w2 {
    width: 52%;
    height: 54%;
    left: 8%;
    bottom: -26%;
    background: var(--glow-lavender);
    animation: drift-b 34s ease-in-out infinite alternate;
  }
  .backdrop.contained .w1 {
    width: 85%;
    height: 85%;
    right: -20%;
    top: -30%;
    background: color-mix(in srgb, var(--accent) 26%, transparent);
  }
  .backdrop.contained .w2 {
    width: 75%;
    height: 75%;
    left: -15%;
    bottom: -30%;
    background: color-mix(in srgb, var(--blush) 22%, transparent);
  }
  @keyframes drift-a {
    to { transform: translate3d(-6%, 8%, 0) scale(1.06); }
  }
  @keyframes drift-b {
    to { transform: translate3d(8%, -6%, 0) scale(1.05); }
  }
  .backdrop.still * { animation: none !important; }
  @media (prefers-reduced-motion: reduce) {
    .backdrop * { animation: none !important; }
  }
</style>
