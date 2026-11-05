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
  .bar { height: 4px; border-radius: 2px; background: var(--surface-3); overflow: hidden; }
  .fill {
    height: 100%; width: 100%; background: var(--accent); border-radius: 2px;
    transform-origin: left; transition: transform var(--dur-3) var(--ease-out);
  }
  .unknown .fill { width: 35%; animation: sweep 1.4s var(--ease-out) infinite; }
  @keyframes sweep { from { transform: translateX(-100%); } to { transform: translateX(300%); } }
  @media (prefers-reduced-motion: reduce) { .unknown .fill { animation: none; width: 100%; opacity: 0.4; } }
</style>
