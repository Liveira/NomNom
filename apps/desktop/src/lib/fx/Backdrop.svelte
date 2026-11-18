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
  const stars = $derived(make(count(30), 11));
  const petals = $derived(make(count(14), 23));
  const bubbles = $derived(make(count(12), 37));
  const flies = $derived(make(count(16), 53));
  const motes = $derived(make(count(10), 71));
  const sparks = $derived(make(count(18), 89));
  const flakes = $derived(make(count(22), 97));
  const leaves = $derived(make(count(12), 101));
  const hearts = $derived(make(count(12), 113));
  const shooters = $derived(make(3, 127));
  const clouds = $derived(make(5, 131));
  const GLYPHS = '01<>/{}[]$#@*+=-~';
  const rain = $derived(
    make(count(16), 139).map((c) => {
      const r = rng(Math.floor(c.s * 1e6) + 7);
      return { ...c, text: Array.from({ length: 22 }, () => GLYPHS[Math.floor(r() * GLYPHS.length)]).join('\n') };
    }),
  );
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
  {:else if theme === 'nebula'}
    <i class="blob b1 neb1"></i><i class="blob b2 neb2"></i><i class="blob b3 neb3"></i>
    {#each stars as s, i (i)}
      <i class="star" style="left:{s.x}%; top:{s.y}%; --s:{1 + s.s * 2.4}px; --d:{2 + s.d * 4}s; --t:{-s.t * 6}s"></i>
    {/each}
    {#if live}
      {#each shooters as s, i (i)}
        <i class="shooter" style="left:{20 + s.x * 0.7}%; top:{s.y * 0.4}%; --d:{7 + s.d * 6}s; --t:{-s.t * 10}s"></i>
      {/each}
    {/if}
  {:else if theme === 'sunset'}
    <i class="skyglow"></i>
    <i class="dusksun"></i>
    {#each clouds as c, i (i)}
      <i class="cloud" style="top:{8 + c.y * 0.5}%; --w:{180 + c.s * 260}px; --d:{60 + c.d * 70}s; --t:{-c.t * 90}s; --o:{0.12 + c.u * 0.16}"></i>
    {/each}
  {:else if theme === 'matcha'}
    <i class="blob b1 mat1"></i><i class="blob b2 mat2"></i>
    {#each leaves as p, i (i)}
      <i class="petal leaf" style="left:{p.x}%; --s:{12 + p.s * 14}px; --d:{11 + p.d * 11}s; --t:{-p.t * 22}s; --sway:{26 + p.u * 50}px; --rot:{p.u * 360}deg"><b></b></i>
    {/each}
  {:else if theme === 'lavender'}
    <i class="blob b1"></i><i class="blob b3"></i>
    {#each motes as m, i (i)}
      <i class="mote pollen" style="left:{m.x}%; top:{30 + m.y * 0.7}%; --s:{5 + m.s * 9}px; --d:{8 + m.d * 10}s; --t:{-m.t * 14}s; --dx:{(m.u - 0.5) * 120}px"></i>
    {/each}
  {:else if theme === 'ember'}
    <i class="heat"></i>
    {#each sparks as p, i (i)}
      <i class="spark" style="left:{p.x}%; --s:{2 + p.s * 4}px; --d:{5 + p.d * 7}s; --t:{-p.t * 12}s; --sway:{(p.u - 0.5) * 120}px"></i>
    {/each}
  {:else if theme === 'frost'}
    <i class="blob b1 ice1"></i><i class="blob b2 ice2"></i>
    {#each flakes as p, i (i)}
      <i class="flake" style="left:{p.x}%; --s:{3 + p.s * 7}px; --d:{9 + p.d * 14}s; --t:{-p.t * 22}s; --sway:{(p.u - 0.5) * 70}px"></i>
    {/each}
  {:else if theme === 'matrix'}
    <i class="crt"></i>
    {#each rain as c, i (i)}
      <i class="code" style="left:{c.x}%; --d:{6 + c.d * 9}s; --t:{-c.t * 15}s; --f:{9 + c.s * 7}px; --o:{0.35 + c.u * 0.55}">{c.text}</i>
    {/each}
  {:else if theme === 'cyberpunk'}
    <i class="neonbar nb1"></i><i class="neonbar nb2"></i><i class="neonbar nb3"></i>
    <i class="glitch"></i>
    <i class="scan"></i>
  {:else if theme === 'candy'}
    <i class="blob b1 can1"></i><i class="blob b2 can2"></i>
    {#each hearts as p, i (i)}
      <i class="bubble candy" style="left:{p.x}%; --s:{10 + p.s * 26}px; --d:{10 + p.d * 14}s; --t:{-p.t * 24}s; --sway:{16 + p.u * 40}px; --h:{p.u * 360}"></i>
    {/each}
  {:else if theme === 'graphite'}
    <i class="beam gb1"></i><i class="beam gb2"></i>
    <i class="grain"></i>
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
  /* Soft edges come from a mask that is painted once. A live blur is redone on every frame, which
     is what made the old background expensive, most of all on Linux. */
  .blob { border-radius: 50%; opacity: 0.9; will-change: transform; -webkit-mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); }
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
  .ribbon { will-change: transform; border-radius: 50%; -webkit-mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); }
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
    opacity: 0.6;
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
  .mist { left: -20%; width: 140%; height: 38%; will-change: transform; }
  .m1 { bottom: -8%; background: radial-gradient(60% 100% at 30% 100%, var(--glow-blush), transparent); animation: mist 40s ease-in-out infinite alternate; }
  .m2 { bottom: 10%; background: radial-gradient(50% 100% at 70% 100%, var(--glow-lavender), transparent); animation: mist 55s ease-in-out infinite alternate-reverse; }
  @keyframes roam {
    0% { transform: translate3d(0, 0, 0); }
    50% { transform: translate3d(var(--ax), var(--ay), 0); }
    100% { transform: translate3d(var(--bx), var(--by), 0); }
  }
  @keyframes blink { 0%, 100% { opacity: 0.1; } 45%, 60% { opacity: 1; } }
  @keyframes mist { to { transform: translateX(8%) scale(1.1); } }

  /* ----- Nebula, Sunset, Matcha, Lavender -------------------------------------------------------- */
  .neb1 { background: color-mix(in srgb, var(--accent) 30%, transparent); }
  .neb2 { background: color-mix(in srgb, var(--blush) 24%, transparent); }
  .neb3 { background: color-mix(in srgb, var(--lavender) 22%, transparent); }
  .shooter {
    width: 130px; height: 2px; border-radius: 2px; opacity: 0;
    background: linear-gradient(90deg, transparent, var(--text));
    transform: rotate(28deg); animation: shoot var(--d) ease-in var(--t) infinite;
  }
  @keyframes shoot { 0%, 86% { opacity: 0; transform: translate3d(0, 0, 0) rotate(28deg); } 90% { opacity: 1; } 100% { opacity: 0; transform: translate3d(36vw, 22vh, 0) rotate(28deg); } }

  .skyglow { inset: 0; background: linear-gradient(180deg, transparent 30%, color-mix(in srgb, var(--blush) 22%, transparent) 78%, color-mix(in srgb, var(--accent) 30%, transparent) 100%); animation: skyshift 18s ease-in-out infinite alternate; }
  .dusksun {
    width: min(34vw, 300px); aspect-ratio: 1; left: 62%; bottom: -8%; border-radius: 50%;
    background: radial-gradient(circle at 50% 40%, #ffe29a, var(--accent) 55%, var(--blush) 100%);
    box-shadow: 0 0 90px var(--accent); opacity: 0.6; animation: sunbob 14s ease-in-out infinite alternate;
  }
  .cloud {
    left: -30%; width: var(--w); height: calc(var(--w) * 0.22); border-radius: 50%; opacity: var(--o);
    background: color-mix(in srgb, var(--text) 70%, var(--blush)); -webkit-mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); mask-image: radial-gradient(closest-side, #000 18%, transparent 100%);
    animation: cloud var(--d) linear var(--t) infinite; will-change: transform;
  }
  @keyframes skyshift { to { opacity: 0.65; } }
  @keyframes sunbob { to { transform: translateY(-6%) scale(1.06); opacity: 0.75; } }
  @keyframes cloud { to { transform: translateX(160vw); } }

  .mat1 { background: color-mix(in srgb, var(--accent) 24%, transparent); }
  .mat2 { background: color-mix(in srgb, var(--blush) 22%, transparent); }
  .leaf b { border-radius: 0 100% 0 100%; background: linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 50%, #2f5d1a)); opacity: 0.75; }
  .pollen { background: var(--lavender); filter: blur(0.5px); box-shadow: 0 0 10px var(--lavender); }

  /* ----- Ember, Frost ----------------------------------------------------------------------------- */
  .heat { left: 0; right: 0; bottom: -12%; height: 46%; background: radial-gradient(70% 100% at 50% 100%, color-mix(in srgb, var(--accent) 42%, transparent), transparent 72%); animation: heat 3.6s ease-in-out infinite alternate; }
  @keyframes heat { to { transform: scaleY(1.18); opacity: 0.75; } }
  .spark {
    bottom: -3%; width: var(--s); height: var(--s); border-radius: 50%;
    background: var(--blush); box-shadow: 0 0 calc(var(--s) * 3) var(--accent);
    animation: spark var(--d) ease-out var(--t) infinite;
  }
  @keyframes spark { 0% { transform: translate3d(0, 0, 0) scale(1); opacity: 0; } 12% { opacity: 1; } 100% { transform: translate3d(var(--sway), -95vh, 0) scale(0.2); opacity: 0; } }

  .ice1 { background: color-mix(in srgb, var(--accent) 24%, transparent); }
  .ice2 { background: color-mix(in srgb, var(--blush) 26%, transparent); }
  .flake {
    top: -4%; width: var(--s); height: var(--s); border-radius: 50%; background: #fff; opacity: 0.85;
    box-shadow: 0 0 6px rgba(120, 170, 220, 0.7); animation: snow var(--d) linear var(--t) infinite;
  }
  @keyframes snow { from { transform: translate3d(0, -4vh, 0); } 50% { transform: translate3d(var(--sway), 54vh, 0); } to { transform: translate3d(calc(var(--sway) * -0.6), 106vh, 0); } }

  /* ----- Matrix, Cyberpunk, Candy, Graphite ------------------------------------------------------- */
  .crt { inset: 0; background: repeating-linear-gradient(180deg, rgba(0, 255, 127, 0.05) 0 1px, transparent 1px 3px); }
  .code {
    top: -60%; width: 1.1em; font: 700 var(--f) / 1.15 var(--font-mono); color: var(--accent); white-space: pre; opacity: var(--o);
    text-shadow: 0 0 8px var(--accent); animation: codefall var(--d) linear var(--t) infinite; will-change: transform;
    -webkit-mask-image: linear-gradient(180deg, transparent, #000 85%); mask-image: linear-gradient(180deg, transparent, #000 85%);
  }
  @keyframes codefall { from { transform: translateY(0); } to { transform: translateY(190vh); } }

  .neonbar { height: 3px; width: 46%; border-radius: 3px; opacity: 0.8; box-shadow: 0 0 10px currentColor; will-change: transform; }
  .nb1 { top: 22%; left: -50%; color: var(--accent); background: currentColor; animation: sweep 7s ease-in-out infinite; }
  .nb2 { top: 54%; right: -50%; color: var(--blush); background: currentColor; animation: sweep2 9s ease-in-out -3s infinite; }
  .nb3 { top: 80%; left: -50%; color: var(--lavender); background: currentColor; animation: sweep 11s ease-in-out -6s infinite; }
  @keyframes sweep { 0% { transform: translateX(0); } 100% { transform: translateX(300%); } }
  @keyframes sweep2 { 0% { transform: translateX(0); } 100% { transform: translateX(-300%); } }
  .glitch { inset: 0; background: linear-gradient(90deg, color-mix(in srgb, var(--blush) 10%, transparent), transparent 30%, transparent 70%, color-mix(in srgb, var(--lavender) 10%, transparent)); animation: glitch 6s steps(1) infinite; }
  @keyframes glitch { 0%, 93%, 100% { opacity: 0.6; transform: none; } 94% { opacity: 1; transform: translateX(-6px) skewX(2deg); } 96% { opacity: 0.8; transform: translateX(8px); } 98% { opacity: 1; transform: translateX(-3px); } }

  .can1 { background: color-mix(in srgb, var(--accent) 24%, transparent); }
  .can2 { background: color-mix(in srgb, var(--blush) 28%, transparent); }
  .candy { border-color: hsl(var(--h) 90% 65% / 0.7); background: radial-gradient(circle at 30% 30%, rgba(255, 255, 255, 0.6), hsl(var(--h) 90% 75% / 0.45) 70%); }

  .beam { width: 60%; height: 140%; top: -20%; -webkit-mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); mask-image: radial-gradient(closest-side, #000 18%, transparent 100%); opacity: 0.22; background: linear-gradient(90deg, transparent, #fff, transparent); will-change: transform; }
  .gb1 { left: -20%; transform: rotate(18deg); animation: beam1 24s ease-in-out infinite alternate; }
  .gb2 { right: -25%; transform: rotate(-16deg); opacity: 0.14; animation: beam2 31s ease-in-out infinite alternate; }
  @keyframes beam1 { to { transform: translateX(70%) rotate(10deg); } }
  @keyframes beam2 { to { transform: translateX(-80%) rotate(-8deg); } }
  .grain { inset: -50%; opacity: 0.05; background-image: radial-gradient(#fff 1px, transparent 1px); background-size: 3px 3px; animation: grain 0.9s steps(4) infinite; }
  @keyframes grain { 0% { transform: translate(0, 0); } 25% { transform: translate(-3px, 2px); } 50% { transform: translate(2px, -3px); } 75% { transform: translate(-2px, -1px); } 100% { transform: translate(0, 0); } }

  .backdrop.still .shooter, .backdrop.still .spark, .backdrop.still .flake, .backdrop.still .code, .backdrop.still .cloud { display: none; }

  /* ----- Calm and Off: the colour is kept, the movement is not ----------------------------------- */
  .backdrop.still * { animation: none !important; }
  .backdrop.still .petal, .backdrop.still .bubble, .backdrop.still .mote, .backdrop.still .firefly { display: none; }
  @media (prefers-reduced-motion: reduce) {
    .backdrop * { animation: none !important; }
  }
</style>
