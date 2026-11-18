<script lang="ts">
  /** `value` is 0..1, or null while the amount is unknown. */
  let { value, label = 'Progress' }: { value: number | null; label?: string } = $props();
</script>

<div
  class="bar"
  class:unknown={value == null}
  role="progressbar"
  aria-label={label}
  aria-valuemin="0"
  aria-valuemax="100"
  aria-valuenow={value == null ? undefined : Math.round(value * 100)}
>
  <div class="fill" style:transform={value == null ? undefined : `scaleX(${value})`}></div>
</div>

<style>
  .bar {
    height: 6px;
    border-radius: var(--radius-pill);
    background: var(--surface-3);
    overflow: hidden;
    box-shadow: inset 0 1px 2px rgba(0, 0, 0, 0.25);
  }
  .fill {
    height: 100%;
    width: 100%;
    background: linear-gradient(90deg, var(--accent), var(--blush));
    border-radius: var(--radius-pill);
    transform-origin: left;
    transition: transform var(--dur-3) var(--ease-out);
  }
  .unknown .fill { width: 35%; animation: sweep 1.4s var(--ease-out) infinite; }
  @keyframes sweep { from { transform: translateX(-100%); } to { transform: translateX(300%); } }
  @media (prefers-reduced-motion: reduce) { .unknown .fill { animation: none; width: 100%; opacity: 0.45; } }
</style>
