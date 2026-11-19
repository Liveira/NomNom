<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Switch from '../../lib/ui/Switch.svelte';

  const s = $derived(app.settings?.activity);
  let error = $state('');

  const idleChoices = [
    { secs: 60, label: '1 minute' },
    { secs: 180, label: '3 minutes' },
    { secs: 300, label: '5 minutes' },
    { secs: 600, label: '10 minutes' },
    { secs: 1800, label: '30 minutes' },
  ];

  const reminderChoices = [
    { hour: 24, label: 'Never' },
    { hour: 18, label: 'At 6 pm' },
    { hour: 20, label: 'At 8 pm' },
    { hour: 21, label: 'At 9 pm' },
    { hour: 22, label: 'At 10 pm' },
  ];

  async function save(patch: Parameters<typeof app.update>[0]) {
    error = '';
    try {
      await app.update(patch);
    } catch (e) {
      error = String(e);
    }
  }
</script>

<section class="group" aria-labelledby="act">
  <h3 id="act">Playtime</h3>

  <div class="row">
    <div>
      <div class="label">Track active app and game hours</div>
      <div class="hint">Counts only the window you are using. Stored on this computer, never sent anywhere.</div>
    </div>
    <Switch checked={s?.enabled ?? true} label="Track active app and game hours" onchange={(v) => save({ activityEnabled: v })} />
  </div>

  <div class="row">
    <div>
      <div class="label">Stop counting after no input for</div>
      <div class="hint">Keyboard, mouse or gamepad. Time spent away from the computer isn't counted.</div>
    </div>
    <select
      aria-label="Stop counting after no input for"
      value={s?.idle_timeout_secs ?? 300}
      onchange={(e) => save({ activityIdleSecs: Number(e.currentTarget.value) })}
    >
      {#each idleChoices as c (c.secs)}<option value={c.secs}>{c.label}</option>{/each}
    </select>
  </div>

  <div class="row">
    <div>
      <div class="label">Count time in NomNom itself</div>
      <div class="hint">Off by default, so the stats show your other apps.</div>
    </div>
    <Switch checked={s?.track_nomnom ?? false} label="Count time in NomNom itself" onchange={(v) => save({ activityTrackNomnom: v })} />
  </div>
  <div class="row">
    <div>
      <div class="label">Remind me about a goal I have not reached</div>
      <div class="hint">A target you are still short of, once a day, at this hour.</div>
    </div>
    <select
      aria-label="Remind me about a goal I have not reached"
      value={s?.goal_reminder_hour ?? 20}
      onchange={(e) => save({ goalReminderHour: Number(e.currentTarget.value) })}
    >
      {#each reminderChoices as c (c.hour)}<option value={c.hour}>{c.label}</option>{/each}
    </select>
  </div>

  <div class="row">
    <div>
      <div class="label">Make a report each week</div>
      <div class="hint">On Monday, with the hours of the week that ended and how your goals went.</div>
    </div>
    <Switch checked={s?.weekly_report ?? true} label="Make a report each week" onchange={(v) => save({ weeklyReport: v })} />
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
</section>

<style>
  .group { max-width: 640px; margin-bottom: 16px; background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 16px 18px; }
  h3 { margin-bottom: 12px; }
  .row { display: flex; justify-content: space-between; align-items: center; gap: 18px; padding: 8px 0; }
  .row + .row { border-top: 1px solid var(--border); }
  .label { font-weight: 500; }
  .hint { color: var(--text-3); font-size: 12.5px; margin-top: 2px; }
  .error { color: var(--danger); margin: 8px 0 0; }
  select { padding: 6px 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
</style>
