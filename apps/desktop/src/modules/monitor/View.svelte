<script lang="ts">
  import { every } from '../../lib/poll';
  import { onDestroy } from 'svelte';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { formatBytes } from '../../lib/format';
  import { monitorIpc, type MonSnapshot } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { load, percentOf, pushSample, rateText, sparkPoints, uptimeText } from './logic';

  let snap = $state<MonSnapshot | null>(null);
  let cpuHist = $state<number[]>([]);
  let memHist = $state<number[]>([]);
  let downHist = $state<number[]>([]);
  let upHist = $state<number[]>([]);
  let by = $state<'cpu' | 'memory'>('cpu');
  let error = $state('');
  let busy = false;

  async function tick() {
    if (busy || document.hidden) return;
    busy = true;
    try {
      const s = await monitorIpc.snapshot();
      snap = s;
      cpuHist = pushSample(cpuHist, s.cpu);
      memHist = pushSample(memHist, percentOf(s.mem_used, s.mem_total));
      downHist = pushSample(downHist, s.net_down);
      upHist = pushSample(upHist, s.net_up);
      error = '';
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
  void tick();
  const timer = every(1500, () => void tick());
  onDestroy(timer);

  const R = 46;
  const C = 2 * Math.PI * R;
  const memPct = $derived(snap ? percentOf(snap.mem_used, snap.mem_total) : 0);
  const procs = $derived(snap ? (by === 'cpu' ? snap.by_cpu : snap.by_memory) : []);
  const maxMem = $derived(Math.max(1, ...(snap?.by_memory.map((p) => p.memory) ?? [1])));
  const netTop = $derived(Math.max(1024 * 100, ...downHist, ...upHist));
  const trayOn = $derived(app.settings?.monitor.tray_tooltip ?? false);
</script>

<header class="studio-hero">
  <div>
    <div class="eyebrow"><Icon name="activity" size={14} /><span>Live, and only looking</span></div>
    <h1>System Monitor</h1>
    <p>See what your computer is doing right now: processor, memory, disks, network and the programs that use the most.</p>
    <div class="tray">
      <Switch checked={trayOn} label="Show CPU and memory when I point at the tray icon" onchange={(v) => app.update({ monitorTray: v })} />
      <span>Show CPU and memory when I point at the tray icon</span>
    </div>
  </div>
  {#if snap}<div class="up"><span>On for</span><strong class="tabular">{uptimeText(snap.uptime_secs)}</strong></div>{/if}
</header>

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if snap}
  <section class="gauges" aria-label="Processor and memory">
    <div class="card g {load(snap.cpu)}">
      <div class="ring" role="img" aria-label={`Processor ${Math.round(snap.cpu)} percent`}>
        <svg viewBox="0 0 110 110" aria-hidden="true">
          <circle class="track" cx="55" cy="55" r={R} />
          <circle class="arc" cx="55" cy="55" r={R} stroke-dasharray={C} stroke-dashoffset={C * (1 - snap.cpu / 100)} />
        </svg>
        <strong class="tabular">{Math.round(snap.cpu)}%</strong>
      </div>
      <div class="gtxt">
        <h2><Icon name="cpu" size={16} /> Processor</h2>
        <p class="sub" translate="no" title={snap.cpu_name}>{snap.cpu_name || '—'}</p>
        <p class="sub tabular">{snap.cores.length} threads{snap.temp_c != null ? ` · ${Math.round(snap.temp_c)} °C` : ''}</p>
        <svg class="spark" viewBox="0 0 200 40" preserveAspectRatio="none" aria-hidden="true"><polyline points={sparkPoints(cpuHist, 200, 40, 100)} /></svg>
      </div>
    </div>

    <div class="card g {load(memPct)}">
      <div class="ring" role="img" aria-label={`Memory ${Math.round(memPct)} percent`}>
        <svg viewBox="0 0 110 110" aria-hidden="true">
          <circle class="track" cx="55" cy="55" r={R} />
          <circle class="arc" cx="55" cy="55" r={R} stroke-dasharray={C} stroke-dashoffset={C * (1 - memPct / 100)} />
        </svg>
        <strong class="tabular">{Math.round(memPct)}%</strong>
      </div>
      <div class="gtxt">
        <h2><Icon name="layers" size={16} /> Memory</h2>
        <p class="sub tabular">{formatBytes(snap.mem_used)} of {formatBytes(snap.mem_total)}</p>
        {#if snap.swap_total > 0}<p class="sub tabular">Swap: {formatBytes(snap.swap_used)} of {formatBytes(snap.swap_total)}</p>{/if}
        <svg class="spark" viewBox="0 0 200 40" preserveAspectRatio="none" aria-hidden="true"><polyline points={sparkPoints(memHist, 200, 40, 100)} /></svg>
      </div>
    </div>

    <div class="card g net">
      <div class="gtxt wide">
        <h2><Icon name="globe" size={16} /> Network</h2>
        <div class="rates">
          <div><span class="k">Down</span><strong class="tabular">{rateText(snap.net_down)}</strong></div>
          <div><span class="k">Up</span><strong class="tabular">{rateText(snap.net_up)}</strong></div>
        </div>
        <svg class="spark two" viewBox="0 0 200 50" preserveAspectRatio="none" aria-hidden="true">
          <polyline class="d" points={sparkPoints(downHist, 200, 50, netTop)} />
          <polyline class="u" points={sparkPoints(upHist, 200, 50, netTop)} />
        </svg>
      </div>
    </div>
  </section>

  <section class="two">
    <div class="card">
      <h2><Icon name="storage" size={16} /> Disks</h2>
      <ul class="disks">
        {#each snap.disks as d (d.mount)}
          {@const pct = percentOf(d.used, d.total)}
          <li class={load(pct)}>
            <div class="dh"><strong translate="no">{d.name || d.mount}</strong><span class="sub tabular" translate="no">{d.mount}</span></div>
            <div class="bar" role="img" aria-label={`${Math.round(pct)} percent used`}><i style:width="{pct}%"></i></div>
            <div class="dn tabular"><span>{formatBytes(d.used)} used</span><span>{formatBytes(d.total - d.used)} free of {formatBytes(d.total)}</span></div>
          </li>
        {/each}
      </ul>
    </div>

    <div class="card">
      <div class="ph">
        <h2><Icon name="gauge" size={16} /> Busiest programs</h2>
        <div class="seg" role="radiogroup" aria-label="Sort by">
          <button role="radio" aria-checked={by === 'cpu'} class:on={by === 'cpu'} onclick={() => (by = 'cpu')}>Processor</button>
          <button role="radio" aria-checked={by === 'memory'} class:on={by === 'memory'} onclick={() => (by = 'memory')}>Memory</button>
        </div>
      </div>
      <ul class="procs">
        {#each procs as p (p.name)}
          <li>
            <span class="pn" translate="no" title={p.name}>{p.name}</span>
            <span class="pbar"><i style:width="{by === 'cpu' ? Math.min(100, p.cpu * 2) : (p.memory / maxMem) * 100}%"></i></span>
            <span class="pv tabular">{by === 'cpu' ? `${p.cpu.toFixed(1)}%` : formatBytes(p.memory)}</span>
          </li>
        {/each}
      </ul>
      <p class="sub note">Programs that run as several processes, like a browser, are added together. NomNom only shows them; it never closes anything.</p>
    </div>
  </section>
{:else}
  <p class="sub">Reading your computer…</p>
{/if}

<style>
  .studio-hero p { max-width: 62ch; }
  .tray { display: flex; align-items: center; gap: 10px; margin-top: 12px; color: var(--text-2); font-size: 13px; }
  .up { display: grid; text-align: right; padding: 10px 16px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border-strong); }
  .up span { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-3); }
  .error { color: var(--danger); }
  .card { padding: 16px 18px; border-radius: var(--radius-l); background: color-mix(in srgb, var(--surface-1) 88%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-soft); }
  h2 { display: flex; align-items: center; gap: 7px; font-size: 15px; margin-bottom: 6px; }
  .sub { color: var(--text-3); font-size: 12.5px; margin: 2px 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .gauges { display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 14px; margin-bottom: 14px; }
  .g { display: grid; grid-template-columns: auto minmax(0, 1fr); gap: 16px; align-items: center; animation: rise var(--dur-4) var(--ease-out) backwards; }
  .g.net { grid-template-columns: 1fr; }
  .ring { position: relative; width: 110px; height: 110px; display: grid; place-items: center; }
  .ring svg { position: absolute; inset: 0; transform: rotate(-90deg); }
  .ring circle { fill: none; stroke-width: 9; }
  .track { stroke: var(--surface-3); }
  .arc { stroke: var(--accent); stroke-linecap: round; transition: stroke-dashoffset 0.9s var(--ease-out), stroke 0.3s; }
  .busy .arc { stroke: var(--lavender); }
  .high .arc { stroke: var(--danger); }
  .ring strong { font: 700 24px var(--font-display); }
  .spark { width: 100%; height: 40px; margin-top: 6px; }
  .spark polyline { fill: none; stroke: var(--accent); stroke-width: 1.8; stroke-linejoin: round; stroke-linecap: round; vector-effect: non-scaling-stroke; }
  .spark.two polyline.d { stroke: var(--accent); }
  .spark.two polyline.u { stroke: var(--blush); }
  .spark.two { height: 50px; }
  .rates { display: flex; gap: 28px; margin: 6px 0; }
  .rates > div { display: grid; }
  .k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.08em; color: var(--text-3); font-weight: 700; }
  .rates strong { font: 700 20px var(--font-display); }
  .two { display: grid; grid-template-columns: repeat(auto-fit, minmax(360px, 1fr)); gap: 14px; }
  .disks, .procs { list-style: none; margin: 8px 0 0; padding: 0; display: grid; gap: 12px; }
  .dh { display: flex; justify-content: space-between; gap: 10px; }
  .bar { height: 8px; border-radius: 4px; background: var(--surface-3); overflow: hidden; margin: 4px 0; }
  .bar i { display: block; height: 100%; border-radius: 4px; background: var(--accent); transition: width 0.6s var(--ease-out); }
  li.busy .bar i { background: var(--lavender); }
  li.high .bar i { background: var(--danger); }
  .dn { display: flex; justify-content: space-between; color: var(--text-3); font-size: 12px; }
  .ph { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; }
  .seg { display: inline-flex; padding: 3px; border-radius: var(--radius-pill); background: var(--surface-2); border: 1px solid var(--border); }
  .seg button { border: 0; background: transparent; color: var(--text-2); padding: 4px 12px; border-radius: var(--radius-pill); cursor: pointer; }
  .seg button.on { background: var(--surface-3); color: var(--text); font-weight: 700; }
  .procs { gap: 8px; }
  .procs li { display: grid; grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr) 76px; gap: 10px; align-items: center; }
  .pn { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .pbar { height: 6px; border-radius: 3px; background: var(--surface-3); overflow: hidden; }
  .pbar i { display: block; height: 100%; background: linear-gradient(90deg, var(--accent), var(--blush)); transition: width 0.6s var(--ease-out); }
  .pv { text-align: right; color: var(--text-2); font-size: 12.5px; }
  .note { white-space: normal; margin-top: 12px; }
</style>
