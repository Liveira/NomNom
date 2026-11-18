<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Switch from '../../lib/ui/Switch.svelte';

  const closeToTray = $derived(app.settings?.general.close_to_tray ?? true);
</script>

<section class="group" aria-labelledby="gen">
  <h3 id="gen">When you close the window</h3>
  <div class="row">
    <div>
      <div class="label">Keep running in the tray</div>
      <div class="hint">Downloads, shortcuts and Playtime keep working. Turn this off to quit instead.</div>
    </div>
    <Switch checked={closeToTray} label="Keep running in the tray when the window is closed" onchange={(v) => app.setCloseToTray(v)} />
  </div>
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
</style>
