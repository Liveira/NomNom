<script lang="ts" generics="T extends string | number">
  type Option = { value: T; label: string };

  let {
    options,
    value,
    label,
    onchange,
    open = $bindable(false),
    disabled = false,
  }: {
    options: Option[];
    value: T;
    label: string;
    onchange: (value: T) => void;
    open?: boolean;
    disabled?: boolean;
  } = $props();

  let active = $state(0);
  let button: HTMLButtonElement | undefined = $state();
  const uid = Math.random().toString(36).slice(2, 8);
  const current = $derived(options.find((o) => o.value === value) ?? options[0]);

  function show() {
    if (disabled) return;
    active = Math.max(0, options.findIndex((o) => o.value === value));
    open = true;
  }

  function choose(i: number) {
    const opt = options[i];
    if (opt) onchange(opt.value);
    open = false;
    button?.focus();
  }

  function onKey(e: KeyboardEvent) {
    if (!open) {
      if (e.key === 'ArrowDown' || e.key === 'ArrowUp' || e.key === ' ') {
        e.preventDefault();
        show();
      }
      return; // Enter and Escape belong to the page when the list is closed.
    }
    switch (e.key) {
      case 'ArrowDown':
        active = Math.min(options.length - 1, active + 1);
        break;
      case 'ArrowUp':
        active = Math.max(0, active - 1);
        break;
      case 'Home':
        active = 0;
        break;
      case 'End':
        active = options.length - 1;
        break;
      case 'Enter':
      case ' ':
        choose(active);
        break;
      case 'Escape':
        open = false;
        break;
      case 'Tab':
        open = false;
        return;
      default:
        return;
    }
    // Keys used by the open list must not also confirm or close the whole popup.
    e.preventDefault();
    e.stopPropagation();
  }
</script>

<div class="wrap">
  <button
    bind:this={button}
    type="button"
    class="trigger"
    role="combobox"
    aria-label={label}
    aria-haspopup="listbox"
    aria-expanded={open}
    aria-controls={`lb-${uid}`}
    {disabled}
    onclick={() => (open ? (open = false) : show())}
    onkeydown={onKey}
    onblur={() => (open = false)}
  >
    <span class="text">{current?.label}</span>
    <svg width="12" height="12" viewBox="0 0 12 12" aria-hidden="true"><path d="m2.5 4.5 3.5 3.5 3.5-3.5" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round" /></svg>
  </button>
  {#if open}
    <ul id={`lb-${uid}`} role="listbox" aria-label={label}>
      {#each options as opt, i (opt.value)}
        <!-- Keyboard use goes through the combobox trigger (ARIA listbox pattern); options are not tab stops. -->
        <!-- svelte-ignore a11y_click_events_have_key_events -->
        <!-- mousedown keeps focus on the trigger so the blur handler doesn't close the list first -->
        <li
          role="option"
          aria-selected={opt.value === value}
          class:active={i === active}
          onmousedown={(e) => e.preventDefault()}
          onclick={() => choose(i)}
          onmousemove={() => (active = i)}
        >
          {opt.label}
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .wrap { position: relative; }
  .trigger {
    width: 100%; display: flex; align-items: center; justify-content: space-between; gap: 8px;
    padding: 8px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong);
    background: var(--surface-2); cursor: pointer; text-align: left;
    transition: background var(--dur-1);
  }
  .trigger:hover:not(:disabled) { background: var(--surface-3); }
  .trigger:disabled { opacity: 0.6; cursor: default; }
  .text { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  ul {
    position: absolute; z-index: 10; left: 0; right: 0; top: calc(100% + 4px); margin: 0; padding: 4px;
    list-style: none; max-height: 168px; overflow: auto; background: var(--surface-2);
    border: 1px solid var(--border-strong); border-radius: var(--radius-s);
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.35); animation: pop var(--dur-1) var(--ease-out);
  }
  li { padding: 6px 8px; border-radius: 4px; cursor: pointer; }
  li.active { background: var(--surface-3); }
  li[aria-selected='true'] { color: var(--accent); }
  @keyframes pop { from { opacity: 0; transform: translateY(-3px); } to { opacity: 1; transform: none; } }
</style>
