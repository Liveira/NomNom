<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { LANGUAGES } from '../../lib/i18n/index.svelte';
  import { toastIpc } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';

  const closeToTray = $derived(app.settings?.general.close_to_tray ?? true);
  const language = $derived(app.settings?.general.language ?? 'auto');
  const pretty = $derived(app.settings?.general.pretty_notifications ?? true);
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

<section class="group" aria-labelledby="notes">
  <h3 id="notes">Notifications</h3>
  <div class="row">
    <div>
      <div class="label">Notifications that look like NomNom</div>
      <div class="hint">A small card in the corner of the screen, in your theme. Turn this off to use the system's own notifications.</div>
    </div>
    <Switch checked={pretty} label="Notifications that look like NomNom" onchange={(v) => app.update({ prettyNotifications: v })} />
  </div>
  <div class="row">
    <div class="hint">See how they look.</div>
    <button class="plain" onclick={() => void toastIpc.test()}>Show an example</button>
  </div>
</section>

<section class="group" aria-labelledby="lang">
  <h3 id="lang">Language</h3>
  <p class="hint lead">The whole app changes at once. Names of your own files and apps stay as they are.</p>
  <div class="langs" role="radiogroup" aria-label="Language">
    {#each LANGUAGES as l (l.id)}
      {@const on = language === l.id}
      <button class="lang" class:on role="radio" aria-checked={on} onclick={() => app.setLanguage(l.id)}>
        <span class="lico"><Icon name="languages" size={18} /></span>
        <span class="ltext"><strong translate={l.id === 'auto' ? undefined : 'no'}>{l.name}</strong><span translate={l.id === 'auto' ? undefined : 'no'}>{l.note}</span></span>
        {#if on}<span class="lcheck" aria-hidden="true"><Icon name="check" size={13} stroke={3} /></span>{/if}
      </button>
    {/each}
  </div>
</section>

<style>
  .lead { margin: 0 0 12px; }
  .langs { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 12px; }
  .lang { position: relative; display: flex; align-items: center; gap: 12px; padding: 12px 14px; text-align: left; cursor: pointer; border-radius: var(--radius-m); border: 1px solid var(--border-strong); background: var(--surface-2); }
  .lang:hover { border-color: var(--border-glow); }
  .lang.on { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .lico { width: 36px; height: 36px; border-radius: 12px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); flex: none; }
  .lang.on .lico { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); }
  .ltext { display: grid; gap: 1px; }
  .ltext span { font-size: 12px; color: var(--text-3); }
  .lcheck { position: absolute; top: 10px; right: 10px; width: 22px; height: 22px; border-radius: 50%; display: grid; place-items: center; background: var(--accent); color: var(--on-accent); }

  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
</style>
