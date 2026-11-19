<script lang="ts">
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import { hwIpc, type HwBattery, type HwDisk, type HwGpu } from '../../lib/ipc';
  import { every } from '../../lib/poll';
  import Switch from '../../lib/ui/Switch.svelte';
  import { batteryLine, useText } from './logic';

  let gpus = $state<HwGpu[]>([]);
  let battery = $state<HwBattery | null>(null);
  let seen = $state(false);
  let disks = $state<HwDisk[] | null>(null);
  let reading = $state(false);
  let error = $state('');

  async function load() {
    if (document.hidden) return;
    try {
      const s = await hwIpc.status();
      gpus = s.gpus;
      battery = s.battery;
    } catch {
      /* optional */
    } finally {
      seen = true;
    }
  }
  void load();
  const timer = every(5000, () => void load());
  onDestroy(timer);

  async function checkDisks() {
    reading = true;
    error = '';
    try {
      disks = await hwIpc.disks();
    } catch (e) {
      error = String(e);
    } finally {
      reading = false;
    }
  }

  const alertsOn = $derived(app.settings?.monitor.disk_alerts ?? true);
  const label = { good: 'Healthy', watch: 'Keep an eye on it', bad: 'Needs attention now' } as const;
</script>

{#if gpus.length > 0 || battery}
  <section class="row" aria-label="Graphics card and battery">
    {#each gpus as g (g.name)}
      <div class="card">
        <h2><Icon name="monitor" size={16} /> Graphics card</h2>
        <p class="name" translate="no">{g.name}</p>
        <dl class="tabular">
          {#if g.load != null}<div><dt>Busy</dt><dd>{Math.round(g.load)}%</dd></div>{/if}
          {#if g.temp_c != null}<div><dt>Temperature</dt><dd>{Math.round(g.temp_c)} °C</dd></div>{/if}
          {#if g.mem_used_mb != null && g.mem_total_mb != null}<div><dt>Video memory</dt><dd>{formatBytes(g.mem_used_mb * 1048576)} of {formatBytes(g.mem_total_mb * 1048576)}</dd></div>{/if}
          {#if g.fan_percent != null}<div><dt>Fan</dt><dd>{Math.round(g.fan_percent)}%</dd></div>{/if}
          {#if g.power_w != null}<div><dt>Power</dt><dd>{g.power_w.toFixed(1)} W</dd></div>{/if}
        </dl>
      </div>
    {/each}
    {#if battery}
      <div class="card">
        <h2><Icon name="plug" size={16} /> Battery</h2>
        <p class="big tabular">{battery.percent}%</p>
        <div class="bar" role="img" aria-label={`${battery.percent} percent`}><i class:low={battery.percent <= 20 && !battery.plugged} style:width="{battery.percent}%"></i></div>
        <p class="sub">{batteryLine(battery)}</p>
      </div>
    {/if}
  </section>
{:else if seen}
  <p class="hint">No graphics card readings: NomNom reads NVIDIA cards through nvidia-smi, and none answered.</p>
{/if}

<section class="card disks-card" aria-label="Disk health">
  <div class="head">
    <h2><Icon name="shield" size={16} /> Disk health</h2>
    <button class="plain" disabled={reading} onclick={() => void checkDisks()}>{reading ? 'Reading…' : disks ? 'Check again' : 'Check the disks'}</button>
  </div>
  <p class="sub">Read from what each disk reports about itself. NomNom only looks.</p>
  {#if error}<p class="bad" role="alert">{error}</p>{/if}
  {#if disks}
    <ul class="list">
      {#each disks as d (d.name)}
        <li class={d.verdict}>
          <div class="top">
            <strong translate="no">{d.name}</strong>
            <span class="badge {d.verdict}">{label[d.verdict]}</span>
          </div>
          <div class="facts tabular">
            {#if d.media && d.media !== 'Unspecified'}<span>{d.media}</span>{/if}
            {#if d.size > 0}<span>{formatBytes(d.size)}</span>{/if}
            {#if d.temp_c != null}<span>{Math.round(d.temp_c)} °C</span>{/if}
            {#if d.wear != null}<span>{Math.round(d.wear)}% worn</span>{/if}
            {#if d.hours != null}<span>{useText(d.hours)}</span>{/if}
          </div>
          {#each d.why as w (w)}<p class="why">{w}</p>{/each}
          {#if d.temp_c == null && d.wear == null && d.hours == null}<p class="sub">This disk does not report details to Windows without administrator rights.</p>{/if}
        </li>
      {/each}
    </ul>
  {/if}
  <div class="alerts">
    <Switch checked={alertsOn} label="Check the disks once a day and tell me if one needs attention" onchange={(v) => app.update({ diskAlerts: v })} />
    <span>Check the disks once a day and tell me if one needs attention</span>
  </div>
</section>

<style>
  .row { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; margin-top: 14px; }
  .card { padding: 16px 18px; border-radius: var(--radius-l); background: color-mix(in srgb, var(--surface-1) 88%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-soft); }
  .disks-card { margin-top: 14px; }
  h2 { display: flex; align-items: center; gap: 7px; font-size: 15px; margin: 0 0 6px; }
  .name { margin: 0 0 8px; font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  dl { display: grid; gap: 4px; margin: 0; }
  dl div { display: flex; justify-content: space-between; gap: 12px; font-size: 13px; }
  dt { color: var(--text-3); }
  dd { margin: 0; font-weight: 600; }
  .big { font: 700 30px var(--font-display); margin: 4px 0; }
  .bar { height: 8px; border-radius: 4px; background: var(--surface-3); overflow: hidden; margin: 4px 0; }
  .bar i { display: block; height: 100%; border-radius: 4px; background: var(--ok); }
  .bar i.low { background: var(--danger); }
  .sub, .hint { color: var(--text-3); font-size: 12.5px; margin: 4px 0; }
  .hint { margin-top: 14px; }
  .head { display: flex; align-items: center; justify-content: space-between; gap: 10px; }
  .plain { padding: 6px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  .plain:hover:not(:disabled) { background: var(--surface-3); }
  .plain:disabled { opacity: 0.6; cursor: default; }
  .plain:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .bad { color: var(--danger); font-size: 13px; }
  .list { list-style: none; margin: 10px 0 0; padding: 0; display: grid; gap: 8px; }
  .list li { padding: 10px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .top { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .badge { padding: 2px 10px; border-radius: 99px; font-size: 12px; font-weight: 700; background: var(--glow-mint); color: var(--ok); }
  .badge.watch { background: var(--glow-peach); color: var(--accent-text); }
  .badge.bad { background: color-mix(in srgb, var(--danger) 18%, transparent); color: var(--danger); }
  .facts { display: flex; gap: 12px; flex-wrap: wrap; color: var(--text-3); font-size: 12.5px; margin-top: 4px; }
  .why { margin: 4px 0 0; color: var(--text-2); font-size: 13px; }
  .alerts { display: flex; align-items: center; gap: 10px; margin-top: 12px; color: var(--text-2); font-size: 13px; }
</style>
