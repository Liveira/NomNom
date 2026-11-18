<script lang="ts">
  /**
   * The animated background of a theme. Only transform and opacity are animated, so it stays on the
   * graphics card, and everything pauses when the window is hidden (see base.css).
   * `contained` draws it inside a small preview card instead of behind the whole window.
   */
  let {
    theme,
    motion = 'full',
    contained = false,
  }: { theme: string; motion?: 'full' | 'calm' | 'off'; contained?: boolean } = $props();

  /** Same numbers every time, so the pattern doesn't jump when the theme is re-applied. */
  function rng(seed: number) {
    let a = seed >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  function make(n: number, seed: number) {
    const r = rng(seed);
    return Array.from({ length: n }, () => ({
      x: r() * 100,
      y: r() * 100,
      s: r(),
      d: r(),
      t: r(),
      u: r(),
    }));
  }

  const live = $derived(motion === 'full');
  const count = (n: number) => (contained ? Math.max(4, Math.round(n / 4)) : n);
  const stars = $derived(make(count(46), 11));
  const petals = $derived(make(count(22), 23));
  const bubbles = $derived(make(count(18), 37));
  const flies = $derived(make(count(26), 53));
  const motes = $derived(make(count(16), 71));
</script>

<div class="backdrop" class:contained class:still={!live} data-theme={theme} data-fx={theme} aria-hidden="true">
  {#if theme === 'dark' || theme === 'light'}
    <i class="blob b1"></i><i class="blob b2"></i><i class="blob b3"></i>
    {#if live}
      {#each motes as m, i (i)}
        <i class="mote" style="left:{m.x}%; top:{m.y}%; --s:{4 + m.s * 7}px; --d:{9 + m.d * 10}s; --t:{-m.t * 12}s; --dx:{(m.u - 0.5) * 80}px"></i>
      {/each}
    {/if}
  {:else if theme === 'aurora'}
    <i class="ribbon r1"></i><i class="ribbon r2"></i><i class="ribbon r3"></i>
    {#each stars as s, i (i)}
      <i class="star" style="left:{s.x}%; top:{s.y * 0.7}%; --s:{1 + s.s * 2}px; --d:{2.5 + s.d * 4}s; --t:{-s.t * 6}s"></i>
    {/each}
  {:else if theme === 'synthwave'}
    <i class="sun"></i>
    <div class="floor"><i class="grid"></i></div>
    <i class="horizon"></i>
    <i class="scan"></i>
    {#each stars as s, i (i)}
      <i class="star" style="left:{s.x}%; top:{s.y * 0.5}%; --s:{1 + s.s * 2}px; --d:{2 + s.d * 3}s; --t:{-s.t * 6}s"></i>
    {/each}
  {:else if theme === 'sakura'}
    <i class="blob b1"></i><i class="blob b2"></i>
    {#each petals as p, i (i)}
      <i
        class="petal"
        style="left:{p.x}%; --s:{9 + p.s * 10}px; --d:{9 + p.d * 9}s; --t:{-p.t * 18}s; --sway:{30 + p.u * 60}px; --rot:{p.u * 360}deg"
        ><b></b></i
      >
    {/each}
  {:else if theme === 'ocean'}
    <i class="rays"></i>
    {#each bubbles as b, i (i)}
      <i class="bubble" style="left:{b.x}%; --s:{4 + b.s * 12}px; --d:{9 + b.d * 12}s; --t:{-b.t * 20}s; --sway:{10 + b.u * 30}px"></i>
    {/each}
    <svg class="wave w1" viewBox="0 0 1200 120" preserveAspectRatio="none"><path d="M0 60 Q150 10 300 60 T600 60 T900 60 T1200 60 V120 H0Z" /></svg>
    <svg class="wave w2" viewBox="0 0 1200 120" preserveAspectRatio="none"><path d="M0 70 Q150 25 300 70 T600 70 T900 70 T1200 70 V120 H0Z" /></svg>
  {:else if theme === 'forest'}
    <i class="mist m1"></i><i class="mist m2"></i>
    {#each flies as f, i (i)}
      <i
        class="firefly"
        style="left:{f.x}%; top:{20 + f.y * 0.7}%; --s:{3 + f.s * 4}px; --d:{11 + f.d * 12}s; --b:{2.2 + f.u * 3}s; --t:{-f.t * 20}s; --ax:{(f.u - 0.5) * 140}px; --ay:{(f.s - 0.5) * 100}px; --bx:{(f.d - 0.5) * 160}px; --by:{(f.t - 0.5) * 120}px"
      ></i>
    {/each}
  {/if}
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 0;
    pointer-events: none;
    overflow: hidden;
    contain: strict;
  }
  .backdrop.contained { position: absolute; border-radius: inherit; }
  .backdrop i, .backdrop b, .backdrop svg { position: absolute; display: block; }

  /* ----- Midnight Mochi and Light: slow drifting colour clouds and floating motes ------------- */
  .blob { border-radius: 50%; filter: blur(70px); opacity: 0.9; will-change: transform; }
  .b1 { width: 46%; height: 52%; left: -8%; top: -14%; background: var(--glow-lavender); animation: drift1 26s ease-in-out infinite alternate; }
  .b2 { width: 42%; height: 48%; right: -10%; bottom: -16%; background: var(--glow-blush); animation: drift2 31s ease-in-out infinite alternate; }
  .b3 { width: 34%; height: 40%; left: 38%; top: -20%; background: var(--glow-peach); animation: drift3 22s ease-in-out infinite alternate; }
  .mote {
    width: var(--s); height: var(--s); border-radius: 50%; background: var(--accent); opacity: 0;
    animation: mote var(--d) ease-in-out var(--t) infinite;
  }
  @keyframes drift1 { to { transform: translate3d(14%, 18%, 0) scale(1.15); } }
  @keyframes drift2 { to { transform: translate3d(-16%, -14%, 0) scale(1.1); } }
  @keyframes drift3 { to { transform: translate3d(-22%, 22%, 0) scale(1.2); } }
  @keyframes mote {
    0% { opacity: 0; transform: translate3d(0, 20px, 0) scale(0.6); }
    30% { opacity: 0.5; }
    100% { opacity: 0; transform: translate3d(var(--dx), -90px, 0) scale(1.1); }
  }

  /* ----- Stars (Aurora, Synthwave) --------------------------------------------------------------- */
  .star {
    width: var(--s); height: var(--s); border-radius: 50%; background: var(--text); opacity: 0.2;
    animation: twinkle var(--d) ease-in-out var(--t) infinite;
  }
  @keyframes twinkle { 0%, 100% { opacity: 0.15; transform: scale(0.8); } 50% { opacity: 0.95; transform: scale(1.5); } }

  /* ----- Aurora: three flowing northern-light ribbons ------------------------------------------- */
  .ribbon { filter: blur(48px); will-change: transform; border-radius: 50%; }
  .r1 { width: 90%; height: 34%; left: -20%; top: 4%; background: linear-gradient(90deg, transparent, var(--accent), transparent); opacity: 0.34; animation: ribbon1 16s ease-in-out infinite alternate; }
  .r2 { width: 80%; height: 30%; right: -22%; top: 20%; background: linear-gradient(90deg, transparent, var(--blush), transparent); opacity: 0.3; animation: ribbon2 21s ease-in-out infinite alternate; }
  .r3 { width: 70%; height: 26%; left: 10%; top: 42%; background: linear-gradient(90deg, transparent, var(--lavender), transparent); opacity: 0.22; animation: ribbon3 25s ease-in-out infinite alternate; }
  @keyframes ribbon1 { 0% { transform: translate3d(-6%, 0, 0) rotate(-6deg) scaleY(0.9); } 100% { transform: translate3d(14%, 24%, 0) rotate(5deg) scaleY(1.35); } }
  @keyframes ribbon2 { 0% { transform: translate3d(8%, 8%, 0) rotate(7deg) scaleY(1.2); } 100% { transform: translate3d(-18%, -6%, 0) rotate(-4deg) scaleY(0.85); } }
  @keyframes ribbon3 { 0% { transform: translate3d(-10%, 0, 0) rotate(3deg); } 100% { transform: translate3d(16%, -16%, 0) rotate(-6deg) scaleY(1.3); } }

  /* ----- Synthwave: striped sun, moving neon grid, scanlines ------------------------------------ */
  .sun {
    --sun-mask: linear-gradient(180deg, #000 0 46%, transparent 46% 50%, #000 50% 62%, transparent 62% 67%, #000 67% 76%, transparent 76% 82%, #000 82% 90%, transparent 90% 95%, #000 95%);
    width: min(46vw, 420px); aspect-ratio: 1; left: 50%; bottom: 24%; margin-left: calc(min(46vw, 420px) / -2);
    border-radius: 50%;
    background: linear-gradient(180deg, #ffe66d 0%, var(--accent) 52%, var(--lavender) 100%);
    -webkit-mask-image: var(--sun-mask);
    mask-image: var(--sun-mask);
    opacity: 0.55; filter: drop-shadow(0 0 40px var(--accent));
    animation: sunpulse 6s ease-in-out infinite alternate;
  }
  .horizon { left: 0; right: 0; bottom: 24%; height: 2px; background: linear-gradient(90deg, transparent, var(--blush), var(--accent), var(--blush), transparent); box-shadow: 0 0 18px var(--accent); }
  .floor { left: 0; right: 0; bottom: 0; height: 24%; overflow: hidden; perspective: 280px; }
  .grid {
    left: -60%; right: -60%; top: 0; height: 260%;
    transform-origin: 50% 0; transform: rotateX(62deg);
    background:
      repeating-linear-gradient(90deg, color-mix(in srgb, var(--blush) 55%, transparent) 0 2px, transparent 2px 64px),
      repeating-linear-gradient(0deg, color-mix(in srgb, var(--accent) 60%, transparent) 0 2px, transparent 2px 48px);
    animation: gridmove 1.6s linear infinite;
    -webkit-mask-image: linear-gradient(180deg, transparent, #000 40%);
    mask-image: linear-gradient(180deg, transparent, #000 40%);
  }
  .scan { inset: 0; background: repeating-linear-gradient(180deg, rgba(255, 255, 255, 0.045) 0 1px, transparent 1px 4px); animation: scanflicker 7s steps(2, end) infinite; opacity: 0.8; }
  @keyframes sunpulse { to { opacity: 0.72; transform: scale(1.04); } }
  @keyframes gridmove { to { transform: rotateX(62deg) translateY(48px); } }
  @keyframes scanflicker { 0%, 92%, 100% { opacity: 0.8; } 94% { opacity: 0.4; } 96% { opacity: 1; } }

  /* ----- Sakura: falling petals ------------------------------------------------------------------ */
  .petal {
    top: -6%; width: var(--s); height: var(--s);
    animation: fall var(--d) linear var(--t) infinite; will-change: transform;
  }
  .petal b {
    inset: 0; border-radius: 150% 0 150% 0;
    background: linear-gradient(135deg, var(--accent), var(--blush));
    opacity: 0.7; transform: rotate(var(--rot));
    animation: sway calc(var(--d) / 3) ease-in-out var(--t) infinite alternate;
  }
  @keyframes fall { from { transform: translate3d(0, -4vh, 0); } to { transform: translate3d(8vw, 108vh, 0); } }
  @keyframes sway { from { transform: translateX(calc(var(--sway) * -1)) rotate(var(--rot)); } to { transform: translateX(var(--sway)) rotate(calc(var(--rot) + 140deg)); } }

  /* ----- Deep Ocean: waves, rising bubbles, light rays ------------------------------------------- */
  .rays { inset: -20% 0 30% 0; background: conic-gradient(from 180deg at 50% -10%, transparent 0 40%, color-mix(in srgb, var(--accent) 16%, transparent) 46%, transparent 52% 60%, color-mix(in srgb, var(--blush) 13%, transparent) 66%, transparent 72%); animation: rays 16s ease-in-out infinite alternate; transform-origin: 50% 0; }
  .wave { left: 0; width: 200%; will-change: transform; }
  .w1 { bottom: 0; height: 17%; fill: color-mix(in srgb, var(--accent) 17%, transparent); animation: wave 14s linear infinite; }
  .w2 { bottom: -2%; height: 14%; fill: color-mix(in srgb, var(--blush) 20%, transparent); animation: wave 9s linear infinite reverse; }
  .bubble {
    bottom: -5%; width: var(--s); height: var(--s); border-radius: 50%;
    border: 1.5px solid color-mix(in srgb, var(--accent) 60%, transparent);
    background: radial-gradient(circle at 30% 30%, rgba(255, 255, 255, 0.35), transparent 60%);
    animation: rise var(--d) ease-in var(--t) infinite;
  }
  @keyframes wave { to { transform: translateX(-50%); } }
  @keyframes rays { to { transform: rotate(7deg) scale(1.08); opacity: 0.65; } }
  @keyframes rise {
    0% { transform: translate3d(0, 0, 0); opacity: 0; }
    10% { opacity: 0.9; }
    50% { transform: translate3d(var(--sway), -55vh, 0); }
    100% { transform: translate3d(calc(var(--sway) * -0.6), -110vh, 0); opacity: 0; }
  }

  /* ----- Fireflies Forest: drifting fireflies and mist ------------------------------------------- */
  .firefly {
    width: var(--s); height: var(--s); border-radius: 50%;
    background: var(--accent); box-shadow: 0 0 calc(var(--s) * 3) var(--accent), 0 0 calc(var(--s) * 7) color-mix(in srgb, var(--accent) 50%, transparent);
    animation: roam var(--d) ease-in-out var(--t) infinite alternate, blink var(--b) ease-in-out var(--t) infinite;
  }
  .mist { left: -20%; width: 140%; height: 38%; filter: blur(40px); will-change: transform; }
  .m1 { bottom: -8%; background: radial-gradient(60% 100% at 30% 100%, var(--glow-blush), transparent); animation: mist 40s ease-in-out infinite alternate; }
  .m2 { bottom: 10%; background: radial-gradient(50% 100% at 70% 100%, var(--glow-lavender), transparent); animation: mist 55s ease-in-out infinite alternate-reverse; }
  @keyframes roam {
    0% { transform: translate3d(0, 0, 0); }
    50% { transform: translate3d(var(--ax), var(--ay), 0); }
    100% { transform: translate3d(var(--bx), var(--by), 0); }
  }
  @keyframes blink { 0%, 100% { opacity: 0.1; } 45%, 60% { opacity: 1; } }
  @keyframes mist { to { transform: translateX(8%) scale(1.1); } }

  /* ----- Calm and Off: the colour is kept, the movement is not ----------------------------------- */
  .backdrop.still * { animation: none !important; }
  .backdrop.still .petal, .backdrop.still .bubble, .backdrop.still .mote, .backdrop.still .firefly { display: none; }
  @media (prefers-reduced-motion: reduce) {
    .backdrop * { animation: none !important; }
  }
</style>
