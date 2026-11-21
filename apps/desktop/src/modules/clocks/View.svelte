<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { DEFAULT_ZONES, allZones, clockIn, clockText, dateIn, isZone, localZone, momentIn, offsetLabel, offsetMinutes, parseDuration, stopwatchText } from './logic';
  import { clocks } from './store.svelte';

  const KEY = 'nomnom.clocks';
  const TABS = [
    { id: 'world', label: 'World clock' },
    { id: 'convert', label: 'Time converter' },
    { id: 'stopwatch', label: 'Stopwatch' },
    { id: 'timer', label: 'Timer' },
  ] as const;
  let tab = $state<(typeof TABS)[number]['id']>('world');

  const here = localZone();
  const zones = allZones();
  let saved = $state<string[]>(read());
  let adding = $state('');
  let problem = $state('');
  let now = $state(new Date());
  const tick = setInterval(() => (now = new Date()), 1000);
  onDestroy(() => clearInterval(tick));

  function read(): string[] {
    try {
      const v = JSON.parse(localStorage.getItem(KEY) ?? 'null');
      if (Array.isArray(v) && v.every((x) => typeof x === 'string')) return v;
    } catch {
      /* the first time, or storage is off */
    }
    return DEFAULT_ZONES;
  }
  function write(list: string[]) {
    saved = list;
    try {
      localStorage.setItem(KEY, JSON.stringify(list));
    } catch {
      /* it just won't be remembered */
    }
  }
  const shown = $derived([here, ...saved.filter((z) => z !== here)]);

  function add() {
    problem = '';
    const z = adding.trim();
    if (!z) return;
    if (!isZone(z)) {
      problem = 'That is not a time zone. Pick one from the list, such as Europe/Lisbon.';
      return;
    }
    if (!shown.includes(z)) write([...saved, z]);
    adding = '';
  }

  // Converter
  const pad = (n: number) => String(n).padStart(2, '0');
  const d0 = new Date();
  let when = $state(`${d0.getFullYear()}-${pad(d0.getMonth() + 1)}-${pad(d0.getDate())}T${pad(d0.getHours())}:${pad(d0.getMinutes())}`);
  let from = $state(here);
  const moment = $derived(momentIn(when, from));

  // Timer
  let timerText = $state('10');
  let timerName = $state('');
  let timerError = $state('');
  const PRESETS = ['5', '10', '25', '45', '1h'];
  const presetLabel = (p: string) => (p.endsWith('h') ? p : `${p} min`);
  async function startTimer() {
    timerError = '';
    const s = parseDuration(timerText);
    if (!s || s < 1) return void (timerError = 'Write how long, for example 10, 25m, 1h30 or 45s.');
    if (s > 7 * 86400) return void (timerError = 'A week is the longest.');
    try {
      await clocks.timerStart(s, timerName.trim() || `Timer: ${clockText(s)}`);
    } catch (e) {
      timerError = String(e);
    }
  }
  const lapGaps = $derived(clocks.laps.map((l, i) => l - (clocks.laps[i + 1] ?? 0)));
</script>

<header class="head">
  <h1><Icon name="clock" size={24} /> Time tools</h1>
  <p class="sub">Clocks around the world, a converter between time zones, a stopwatch and a timer. The timer rings with a card even if NomNom's window is closed.</p>
</header>

<div class="tabs" role="tablist" aria-label="Tool">
  {#each TABS as t (t.id)}
    <button role="tab" aria-selected={tab === t.id} class:on={tab === t.id} onclick={() => (tab = t.id)}>{t.label}</button>
  {/each}
</div>

<section class="pane" role="tabpanel">
  {#if tab === 'world'}
    <ul class="zones">
      {#each shown as z (z)}
        <li>
          <div class="where"><strong translate="no">{z.replace(/_/g, ' ')}</strong><span class="hint">{offsetLabel(offsetMinutes(z, now))}{z === here ? ' · this computer' : ''}</span></div>
          <div class="when"><span class="big tabular">{clockIn(z, now)}</span><span class="hint">{dateIn(z, now)}</span></div>
          {#if z !== here}<button class="plain small" aria-label={`Remove ${z}`} onclick={() => write(saved.filter((x) => x !== z))}><Icon name="x" size={14} /></button>{:else}<span></span>{/if}
        </li>
      {/each}
    </ul>
    <form class="row" onsubmit={(e) => { e.preventDefault(); add(); }}>
      <input list="zone-list" bind:value={adding} placeholder="Add a place, such as Europe/Lisbon" aria-label="Add a time zone" spellcheck="false" translate="no" />
      <datalist id="zone-list">{#each zones as z (z)}<option value={z}></option>{/each}</datalist>
      <button class="plain" type="submit">Add</button>
    </form>
    {#if problem}<p class="bad" role="alert">{problem}</p>{/if}
  {:else if tab === 'convert'}
    <div class="row">
      <label>Date and time <input type="datetime-local" bind:value={when} /></label>
      <label>In
        <select bind:value={from} translate="no">
          {#each [...new Set([here, ...zones])] as z (z)}<option value={z}>{z.replace(/_/g, ' ')}</option>{/each}
        </select>
      </label>
    </div>
    {#if moment}
      <ul class="zones">
        {#each shown as z (z)}
          <li>
            <div class="where"><strong translate="no">{z.replace(/_/g, ' ')}</strong><span class="hint">{offsetLabel(offsetMinutes(z, moment))}</span></div>
            <div class="when"><span class="big tabular">{clockIn(z, moment)}</span><span class="hint">{dateIn(z, moment)}</span></div>
            <span></span>
          </li>
        {/each}
      </ul>
    {:else}<p class="bad" role="alert">Pick a date and a time.</p>{/if}
  {:else if tab === 'stopwatch'}
    <div class="dial tabular" aria-live="off">{stopwatchText(clocks.swMs)}</div>
    <div class="row">
      <button class="primary" onclick={() => clocks.swToggle()}>{clocks.swRunning ? 'Stop' : clocks.swMs > 0 ? 'Continue' : 'Start'}</button>
      <button class="plain" disabled={!clocks.swRunning} onclick={() => clocks.swLap()}>Lap</button>
      <button class="plain" disabled={clocks.swMs === 0} onclick={() => clocks.swReset()}>Reset</button>
    </div>
    {#if clocks.laps.length > 0}
      <ol class="laps">
        {#each clocks.laps as l, i (clocks.laps.length - i)}
          <li><span>Lap {clocks.laps.length - i}</span><span class="tabular">{stopwatchText(lapGaps[i])}</span><span class="hint tabular">{stopwatchText(l)}</span></li>
        {/each}
      </ol>
    {/if}
  {:else}
    {#if clocks.timerDue !== null}
      <div class="dial tabular" role="timer">{clockText(clocks.timerLeft)}</div>
      <p class="hint" translate="no">{clocks.timerLabel}</p>
      <div class="row"><button class="plain" onclick={() => void clocks.timerCancel()}>Cancel the timer</button></div>
    {:else}
      <div class="row">
        <label class="grow">How long <input bind:value={timerText} spellcheck="false" translate="no" /></label>
        <label class="grow">What for <span class="opt">(optional)</span> <input bind:value={timerName} placeholder="Tea" maxlength="80" /></label>
      </div>
      <div class="row wrap">
        {#each PRESETS as p (p)}<button class="chip" translate="no" onclick={() => (timerText = p)}>{presetLabel(p)}</button>{/each}
      </div>
      <div class="row"><button class="primary" onclick={() => void startTimer()}>Start the timer</button></div>
      {#if timerError}<p class="bad" role="alert">{timerError}</p>{/if}
    {/if}
  {/if}
</section>

<style>
  .head { display: grid; gap: 8px; margin-bottom: 16px; max-width: 760px; }
  h1 { display: flex; align-items: center; gap: 10px; font-size: 26px; }
  .sub, .hint { color: var(--text-3); font-size: 13px; margin: 0; }
  .opt { color: var(--text-3); font-weight: 500; }
  .bad { color: var(--danger); font-size: 13px; }
  .tabs { display: flex; flex-wrap: wrap; gap: 4px; padding: 3px; border-radius: 18px; background: var(--surface-2); border: 1px solid var(--border); width: fit-content; max-width: 100%; margin-bottom: 16px; }
  .tabs button { border: 0; background: none; color: var(--text-2); padding: 6px 14px; border-radius: 99px; cursor: pointer; font: inherit; font-weight: 600; font-size: 13px; }
  .tabs button.on { background: var(--surface-3); color: var(--text); }
  .pane { display: grid; gap: 12px; max-width: 760px; }
  .zones { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .zones li { display: grid; grid-template-columns: 1fr auto 34px; gap: 12px; align-items: center; padding: 10px 14px; border-radius: var(--radius-s); background: var(--surface-1); border: 1px solid var(--border); }
  .where, .when { display: grid; gap: 2px; }
  .when { text-align: right; }
  .big { font: 600 22px var(--font-display); }
  .row { display: flex; gap: 8px; align-items: flex-end; }
  .row.wrap { flex-wrap: wrap; align-items: center; }
  label { display: grid; gap: 5px; font-weight: 600; font-size: 13px; }
  label.grow { flex: 1; }
  input, select { font: inherit; padding: 9px 12px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); min-width: 0; box-sizing: border-box; }
  .row > input { flex: 1; }
  button:focus-visible, input:focus-visible, select:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  button.plain, button.primary { display: inline-flex; align-items: center; padding: 8px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font: inherit; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button.small { padding: 5px 8px; }
  button:disabled { opacity: 0.5; cursor: default; }
  .chip { border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text-2); border-radius: 99px; padding: 4px 12px; font: inherit; font-size: 12.5px; cursor: pointer; }
  .chip:hover { background: var(--surface-3); color: var(--text); }
  .dial { font: 700 56px/1.1 var(--font-display); padding: 18px 0 6px; letter-spacing: 0.02em; }
  .laps { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; max-width: 420px; }
  .laps li { display: grid; grid-template-columns: 1fr auto auto; gap: 16px; padding: 6px 12px; border-radius: var(--radius-xs); background: var(--surface-1); border: 1px solid var(--border); }
</style>
