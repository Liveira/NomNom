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
    flex: none;
    width: 38px;
    height: 22px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border-strong);
    padding: 2px;
    background: var(--surface-2);
    cursor: pointer;
    transition: background var(--dur-2) var(--ease-out), border-color var(--dur-2) var(--ease-out);
  }
  .switch span {
    display: block;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--text-2);
    transition: transform var(--dur-2) var(--ease-out), background var(--dur-2);
  }
  .switch:focus-visible { border-radius: var(--radius-pill); }
  .switch.on {
    background: var(--accent);
    border-color: var(--accent);
  }
  .switch.on span {
    transform: translateX(16px);
    background: var(--on-accent);
  }
</style>
