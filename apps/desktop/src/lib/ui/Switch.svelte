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
    width: 44px;
    height: 26px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border-strong);
    padding: 3px;
    background: var(--surface-2);
    box-shadow: inset 0 2px 4px rgba(0, 0, 0, 0.25);
    cursor: pointer;
    transition: background var(--dur-2) var(--ease-out), border-color var(--dur-2) var(--ease-out);
  }
  .switch span {
    display: block;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--text-2);
    box-shadow: 0 2px 5px rgba(0, 0, 0, 0.35);
    transition: transform var(--dur-2) var(--ease-spring), background var(--dur-2);
  }
  .switch:focus-visible { border-radius: var(--radius-pill); }
  .switch.on {
    background: linear-gradient(135deg, var(--accent), var(--blush));
    border-color: transparent;
  }
  .switch.on span {
    transform: translateX(18px);
    background: var(--on-accent);
  }
</style>
