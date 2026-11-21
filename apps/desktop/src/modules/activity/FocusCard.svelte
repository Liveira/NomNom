<script lang="ts">
  import { every } from '../../lib/poll';
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
  const timer = every(1000, () => (nowSec = Math.floor(Date.now() / 1000)));
  onDestroy(() => {
    stop?.();
    timer();
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
    display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 16px 22px; align-items: center;
    margin-bottom: 14px; padding: 16px 18px; border-radius: var(--radius-m);
    background: var(--surface-1); border: 1px solid var(--border); box-shadow: var(--shadow-card);
  }
  .focus.running { border-color: var(--accent); }
  .dial { position: relative; width: 116px; height: 116px; display: grid; place-items: center; }
  svg { position: absolute; inset: 0; transform: rotate(-90deg); }
  circle { fill: none; stroke-width: 7; }
  .track { stroke: var(--surface-3); }
  .arc { stroke: var(--accent); stroke-linecap: round; transition: stroke-dashoffset 1s linear; }
  .onbreak .arc { stroke: var(--ok); }
  .time { font: 700 22px var(--font-display); letter-spacing: -0.02em; }
  .sub { position: absolute; bottom: 25px; font-size: 10.5px; color: var(--text-3); text-transform: uppercase; letter-spacing: 0.08em; }
  h2 { display: flex; align-items: center; gap: 7px; font-size: 15.5px; }
  .hint { margin: 4px 0 10px; color: var(--text-2); font-size: 13px; max-width: 54ch; }
  .presets { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-bottom: 10px; }
  .chip { padding: 5px 11px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text-2); cursor: pointer; font-size: 12.5px; font-weight: 600; }
  .chip.on { background: var(--accent-soft); color: var(--accent-text); border-color: var(--accent); }
  .custom { display: inline-flex; align-items: center; gap: 6px; color: var(--text-3); font-size: 12.5px; }
  .custom input { width: 60px; padding: 5px 8px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .what { width: min(360px, 100%); padding: 7px 10px; margin-bottom: 10px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); user-select: text; font-size: 13px; }
  .row { display: flex; gap: 8px; }
  button.primary, button.plain { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: var(--radius-s); cursor: pointer; border: 1px solid var(--border-strong); font-size: 13px; font-weight: 600; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  button.primary:hover { background: var(--accent-hover); }
  button.plain { background: var(--surface-2); color: inherit; }
  button.plain:hover { background: var(--surface-3); }
  .error { color: var(--danger); margin: 8px 0 0; }
  .stats { display: grid; gap: 8px; text-align: center; }
  .stat { display: grid; padding: 7px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); min-width: 96px; }
  .stat strong { display: inline-flex; justify-content: center; align-items: center; gap: 5px; font: 700 16px var(--font-display); }
  .stat span { font-size: 11px; color: var(--text-3); }
  .recent { grid-column: 1 / -1; list-style: none; margin: 0; padding: 12px 0 0; border-top: 1px solid var(--border); display: grid; grid-template-columns: repeat(auto-fit, minmax(240px, 1fr)); gap: 8px 16px; }
  .recent li { display: flex; gap: 9px; align-items: flex-start; }
  .dot { width: 8px; height: 8px; margin-top: 6px; border-radius: 50%; background: var(--accent); flex: none; }
  .dot.brk { background: var(--ok); }
  .what2 { display: grid; font-size: 12.5px; min-width: 0; }
  .lab, .apps { color: var(--text-3); font-size: 12px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  @media (max-width: 900px) { .focus { grid-template-columns: auto 1fr; } .stats { grid-column: 1 / -1; grid-template-columns: repeat(3, 1fr); } }
</style>
