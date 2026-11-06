<script lang="ts">
  let { checked, label, onchange }: { checked: boolean; label: string; onchange: (next: boolean) => void } = $props();
</script>

<button
  type="button"
  class="switch"
  class:on={checked}
  role="switch"
  aria-checked={checked}
  aria-label={label}
  onclick={() => onchange(!checked)}><span></span></button
>

<style>
  .switch {
    flex: none; width: 40px; height: 24px; border-radius: 12px; border: 0; padding: 3px;
    background: var(--surface-3); cursor: pointer; transition: background var(--dur-2);
  }
  .switch span {
    display: block; width: 18px; height: 18px; border-radius: 50%; background: var(--text-2);
    transition: transform var(--dur-2) var(--ease-spring), background var(--dur-2);
  }
  .switch:focus-visible { border-radius: 12px; }
  .switch.on { background: var(--accent); }
  .switch.on span { transform: translateX(16px); background: var(--on-accent); }
</style>
