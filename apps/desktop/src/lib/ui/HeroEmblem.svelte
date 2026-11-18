<script lang="ts">
  import Icon from '../icons/Icon.svelte';

  /** A glowing emblem for page headers: one big icon with small ones orbiting it. */
  let {
    icon,
    orbit = ['sparkle', 'star', 'heart'],
    size = 96,
  }: { icon: string; orbit?: string[]; size?: number } = $props();
</script>

<div class="emblem" style:width="{size}px" style:height="{size}px" aria-hidden="true">
  <div class="ring"></div>
  <div class="core"><Icon name={icon} size={Math.round(size * 0.42)} stroke={2} /></div>
  {#each orbit as o, i (o + i)}
    <span class="sat" style:--i={i} style:--n={orbit.length}><Icon name={o} size={14} stroke={2.2} /></span>
  {/each}
</div>

<style>
  .emblem { position: relative; flex: none; display: grid; place-items: center; }
  .core {
    width: 66%; height: 66%; border-radius: 30%; display: grid; place-items: center;
    color: var(--on-accent);
    background: linear-gradient(145deg, var(--accent), var(--blush));
    box-shadow: 0 8px 26px var(--glow-peach), inset 0 1px 0 rgba(255, 255, 255, 0.4);
    animation: bob 4.6s ease-in-out infinite;
  }
  .ring {
    position: absolute; inset: 0; border-radius: 50%;
    border: 1px dashed var(--border-strong);
    animation: spin 28s linear infinite;
  }
  .sat {
    position: absolute; left: 50%; top: 50%; width: 22px; height: 22px; margin: -11px 0 0 -11px;
    display: grid; place-items: center; border-radius: 50%;
    color: var(--accent-text); background: var(--surface-3); border: 1px solid var(--border-strong);
    --a: calc(360deg / var(--n) * var(--i));
    animation: orbit 14s linear infinite;
    animation-delay: calc(var(--i) * -4.6s);
    transform: rotate(var(--a)) translateX(calc(var(--s, 48px))) rotate(calc(var(--a) * -1));
  }
  .emblem { --s: 50%; }
  @keyframes bob { 0%, 100% { transform: translateY(0) rotate(-2deg); } 50% { transform: translateY(-6px) rotate(2deg); } }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes orbit {
    from { transform: rotate(0deg) translateX(46px) rotate(0deg); }
    to { transform: rotate(360deg) translateX(46px) rotate(-360deg); }
  }
</style>
