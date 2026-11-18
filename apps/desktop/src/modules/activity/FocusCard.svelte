<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { focusIpc, onFocusChanged, type FocusStatus } from '../../lib/ipc';
  import { clock, formatHours, ringOffset } from './focus';

  let status = $state<FocusStatus | null>(null);
  let error = $state('');
  let minutes = $state(25);
  let label = $state('');
  let kind = $state<'focus' | 'break'>('focus');
  let nowSec = $state(Math.floor(Date.now() / 1000));

  const PRESETS: { kind: 'focus' | 'break'; minutes: number; name: string }[] = [
    { kind: 'focus', minutes: 25, name: 'Focus 25' },
    { kind: 'focus', minutes: 50, name: 'Focus 50' },
    { kind: 'break', minutes: 5, name: 'Break 5' },
    { kind: 'break', minutes: 15, name: 'Break 15' },
  ];

  async function refresh() {
    try {
      status = await focusIpc.status();
    } catch (e) {
      error = String(e);
    }
  }
  void refresh();

  let stop: (() => void) | null = null;
  void onFocusChanged(() => void refresh()).then((u) => (stop = u));
  const timer = setInterval(() => (nowSec = Math.floor(Date.now() / 1000)), 1000);
  onDestroy(() => {
    stop?.();
    clearInterval(timer);
  });

  const active = $derived(status?.active ?? null);
  const remaining = $derived(active ? Math.max(0, active.started_at + active.planned_secs - nowSec) : 0);
  const fraction = $derived(active ? remaining / active.planned_secs : 1);

  async function start(k = kind, m = minutes) {
    error = '';
    try {
      await focusIpc.start(k, m, k === 'focus' ? label : '');
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
  async function end() {
    error = '';
    try {
      await focusIpc.stop();
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
  function choose(p: (typeof PRESETS)[number]) {
    kind = p.kind;
    minutes = p.minutes;
  }
  const R = 54;
  const C = 2 * Math.PI * R;
</script>

<section class="focus" class:running={!!active} class:onbreak={active?.kind === 'break'} aria-labelledby="focus-h">
  <div class="dial" role="timer" aria-live="off" aria-label="Time left">
    <svg viewBox="0 0 128 128" aria-hidden="true">
      <circle class="track" cx="64" cy="64" r={R} />
      <circle class="arc" cx="64" cy="64" r={R} stroke-dasharray={C} stroke-dashoffset={ringOffset(C, fraction)} />
    </svg>
    <div class="time tabular">{clock(active ? remaining : minutes * 60)}</div>
    <div class="sub">{active ? (active.kind === 'focus' ? 'Focus' : 'Break') : 'Ready'}</div>
  </div>

  <div class="body">
    <h2 id="focus-h"><Icon name="timer" size={18} /> Focus mode</h2>
    {#if active}
      <p class="hint">
        {#if active.kind === 'focus'}
          {active.label ? `Working on: ${active.label}` : 'Stay with one thing. NomNom tells you when time is up.'}
        {:else}
          Step away for a moment. Your next block is one click away.
        {/if}
      </p>
      <div class="row">
        <button class="plain" onclick={end}><Icon name="pause" size={15} /> Stop now</button>
      </div>
    {:else}
      <p class="hint">Time a block of work or a break. When it ends you see which apps you actually used.</p>
      <div class="presets" role="radiogroup" aria-label="Timer length">
        {#each PRESETS as p (p.name)}
          <button class="chip" role="radio" aria-checked={kind === p.kind && minutes === p.minutes} class:on={kind === p.kind && minutes === p.minutes} onclick={() => choose(p)}>{p.name}</button>
        {/each}
        <label class="custom">
          <input type="number" min="1" max="240" bind:value={minutes} aria-label="Minutes" />
          <span>min</span>
        </label>
      </div>
      {#if kind === 'focus'}
        <input class="what" bind:value={label} maxlength="60" placeholder="What are you working on? (optional)" aria-label="What are you working on?" />
      {/if}
      <div class="row">
        <button class="primary" onclick={() => start()}><Icon name="play" size={15} /> Start {kind === 'focus' ? 'focus' : 'break'}</button>
      </div>
    {/if}
    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </div>

  <div class="stats">
    <div class="stat"><strong class="tabular">{status?.today_blocks ?? 0}</strong><span>blocks today</span></div>
    <div class="stat"><strong class="tabular">{formatHours(status?.today_secs ?? 0)}</strong><span>focused today</span></div>
    <div class="stat"><strong class="tabular"><Icon name="flame" size={16} /> {status?.streak_days ?? 0}</strong><span>day streak</span></div>
  </div>

  {#if (status?.recent.length ?? 0) > 0}
    <ul class="recent" aria-label="Recent blocks">
      {#each (status?.recent ?? []).slice(0, 4) as b (b.id)}
        <li>
          <span class="dot" class:brk={b.kind === 'break'}></span>
          <span class="what2">
            <strong>{b.kind === 'focus' ? 'Focus' : 'Break'} · {formatHours(Math.max(0, b.ended_at - b.started_at))}{b.completed ? '' : ' (stopped early)'}</strong>
            {#if b.label}<span class="lab" translate="no">{b.label}</span>{/if}
            {#if b.apps.length > 0}<span class="apps" translate="no">{b.apps.map((a) => a.name).join(' · ')}</span>{/if}
          </span>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .focus {
    display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 18px 24px; align-items: center;
    margin-bottom: 18px; padding: 18px 22px; border-radius: var(--radius-l);
    background: color-mix(in srgb, var(--surface-1) 86%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-card);
  }
  .focus.running { border-color: var(--border-glow); box-shadow: var(--shadow-card), 0 0 34px var(--glow-peach); }
  .dial { position: relative; width: 132px; height: 132px; display: grid; place-items: center; }
  svg { position: absolute; inset: 0; transform: rotate(-90deg); }
  circle { fill: none; stroke-width: 9; }
  .track { stroke: var(--surface-3); }
  .arc { stroke: var(--accent); stroke-linecap: round; transition: stroke-dashoffset 1s linear; filter: drop-shadow(0 0 6px var(--glow-peach)); }
  .onbreak .arc { stroke: var(--ok); }
  .time { font: 700 26px var(--font-display); letter-spacing: 0.02em; }
  .sub { position: absolute; bottom: 30px; font-size: 11px; color: var(--text-3); text-transform: uppercase; letter-spacing: 0.1em; }
  h2 { display: flex; align-items: center; gap: 8px; font-size: 17px; }
  .hint { margin: 6px 0 12px; color: var(--text-2); font-size: 13px; max-width: 54ch; }
  .presets { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; margin-bottom: 10px; }
  .chip { padding: 6px 14px; border-radius: var(--radius-pill); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text-2); cursor: pointer; }
  .chip.on { background: var(--surface-3); color: var(--text); border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .custom { display: inline-flex; align-items: center; gap: 6px; color: var(--text-3); }
  .custom input { width: 64px; padding: 6px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .what { width: min(380px, 100%); padding: 8px 12px; margin-bottom: 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); user-select: text; }
  .row { display: flex; gap: 10px; }
  button.primary, button.plain { display: inline-flex; align-items: center; gap: 7px; padding: 8px 18px; border-radius: var(--radius-s); cursor: pointer; border: 1px solid var(--border-strong); }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button.plain { background: var(--surface-2); color: inherit; }
  .error { color: var(--danger); margin: 8px 0 0; }
  .stats { display: grid; gap: 10px; text-align: center; }
  .stat { display: grid; padding: 8px 14px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border); min-width: 104px; }
  .stat strong { display: inline-flex; justify-content: center; align-items: center; gap: 5px; font: 700 18px var(--font-display); }
  .stat span { font-size: 11.5px; color: var(--text-3); }
  .recent { grid-column: 1 / -1; list-style: none; margin: 0; padding: 12px 0 0; border-top: 1px solid var(--border); display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 8px 18px; }
  .recent li { display: flex; gap: 10px; align-items: flex-start; }
  .dot { width: 9px; height: 9px; margin-top: 6px; border-radius: 50%; background: var(--accent); flex: none; }
  .dot.brk { background: var(--ok); }
  .what2 { display: grid; font-size: 13px; min-width: 0; }
  .lab, .apps { color: var(--text-3); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 900px) { .focus { grid-template-columns: auto 1fr; } .stats { grid-column: 1 / -1; grid-template-columns: repeat(3, 1fr); } }
</style>
