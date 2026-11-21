<script lang="ts">
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { actIpc, goalsIpc, onGoalsChanged, type ActAppRow, type GoalView, type PlayGoal, type WeekReport } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { CATEGORIES, formatHours } from './logic';
  import { changePercent, dayName, describeGoal, fillOf, sinceLastWeek } from './goals';

  let goals = $state<GoalView[]>([]);
  let apps = $state<ActAppRow[]>([]);
  let thisWeek = $state<WeekReport | null>(null);
  let lastWeek = $state<WeekReport | null>(null);
  let showWeek = $state<'this' | 'last'>('this');
  let editing = $state<PlayGoal | null>(null);
  let hours = $state(2);
  let mins = $state(0);
  let error = $state('');

  const week = $derived(showWeek === 'this' ? thisWeek : lastWeek);
  const maxDay = $derived(Math.max(1, ...(week?.days ?? [1])));

  async function refresh() {
    try {
      goals = await goalsIpc.list();
      thisWeek = await goalsIpc.thisWeek();
      lastWeek = await goalsIpc.report();
    } catch (e) {
      error = String(e);
    }
  }
  void refresh();
  void actIpc.overview('year', false).then((o) => (apps = o.apps.filter((a) => !a.hidden))).catch(() => {});
  let stop: (() => void) | null = null;
  void onGoalsChanged(() => void refresh()).then((u) => (stop = u));
  onDestroy(() => stop?.());

  function add() {
    error = '';
    hours = 2;
    mins = 0;
    editing = { id: '', scope: 'all', kind: 'limit', period: 'day', minutes: 120, enabled: true };
  }
  function edit(g: PlayGoal) {
    error = '';
    hours = Math.floor(g.minutes / 60);
    mins = g.minutes % 60;
    editing = { ...g };
  }
  async function save() {
    if (!editing) return;
    editing.minutes = Math.max(0, Math.floor(hours)) * 60 + Math.max(0, Math.floor(mins));
    try {
      await goalsIpc.save(editing);
      editing = null;
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
  async function toggle(g: PlayGoal, on: boolean) {
    await goalsIpc.save({ ...g, enabled: on });
    await refresh();
  }
  async function remove(id: string) {
    await goalsIpc.remove(id);
    await refresh();
  }
  const scopeLabel = (scope: string) =>
    scope === 'all' ? 'All apps' : scope.startsWith('category:') ? (CATEGORIES.find((c) => c.id === scope.slice(9))?.label ?? scope.slice(9)) : (apps.find((a) => `app:${a.id}` === scope)?.custom_name ?? apps.find((a) => `app:${a.id}` === scope)?.name ?? scope);
</script>

<section class="goals" aria-labelledby="goals-title">
  <div class="head">
    <div>
      <h2 id="goals-title">Goals</h2>
      <p class="sub">Set a limit for a game or an app, or a target for the time you want to give to one.</p>
    </div>
    <button class="plain" onclick={add}><Icon name="plus" size={15} /> Add a goal</button>
  </div>

  {#if error}<p class="err" role="alert">{error}</p>{/if}

  {#if editing}
    <form class="editor" onsubmit={(e) => { e.preventDefault(); void save(); }}>
      <div class="kinds" role="radiogroup" aria-label="Kind of goal">
        <button type="button" role="radio" aria-checked={editing.kind === 'limit'} class:on={editing.kind === 'limit'} onclick={() => editing && (editing.kind = 'limit')}>
          <strong>Limit</strong><span>Stay under this time</span>
        </button>
        <button type="button" role="radio" aria-checked={editing.kind === 'target'} class:on={editing.kind === 'target'} onclick={() => editing && (editing.kind = 'target')}>
          <strong>Target</strong><span>Reach at least this time</span>
        </button>
      </div>
      <div class="fields">
        <label>
          <span>For</span>
          <select bind:value={editing.scope}>
            <option value="all">All apps</option>
            <optgroup label="Kinds of apps">
              {#each CATEGORIES as c (c.id)}<option value={`category:${c.id}`}>{c.label}</option>{/each}
            </optgroup>
            <optgroup label="Apps and games">
              {#each apps as a (a.id)}<option value={`app:${a.id}`} translate="no">{a.custom_name ?? a.name}</option>{/each}
            </optgroup>
          </select>
        </label>
        <label>
          <span>Every</span>
          <select bind:value={editing.period}>
            <option value="day">day</option>
            <option value="week">week</option>
          </select>
        </label>
        <label class="time">
          <span>Time</span>
          <span class="pair"><input type="number" min="0" max="167" bind:value={hours} aria-label="Hours" /> h <input type="number" min="0" max="59" bind:value={mins} aria-label="Minutes" /> min</span>
        </label>
      </div>
      <div class="actions">
        <button type="button" class="plain" onclick={() => (editing = null)}>Cancel</button>
        <button type="submit" class="primary">Save goal</button>
      </div>
    </form>
  {/if}

  {#if goals.length === 0 && !editing}
    <p class="empty">No goals yet. Try “Clip Studio, at least 2 hours a day”, or “Games, at most 3 hours a day”.</p>
  {/if}

  <ul class="list">
    {#each goals as v (v.goal.id)}
      <li class="goal {v.progress.state}" class:off={!v.goal.enabled}>
        <span class="ico">{#if v.icon_url}<img src={v.icon_url} alt="" />{:else}<Icon name={v.goal.kind === 'limit' ? 'clock' : 'flame'} size={18} />{/if}</span>
        <div class="body">
          <div class="row1">
            <strong translate="no">{v.name}</strong>
            <span class="what">{describeGoal(v.goal)}</span>
          </div>
          <div class="bar" role="progressbar" aria-valuemin="0" aria-valuemax="100" aria-valuenow={fillOf(v.progress)} aria-label={`${v.name}: ${fillOf(v.progress)}%`}>
            <i style:width="{fillOf(v.progress)}%"></i>
          </div>
          <div class="row2 tabular">
            <span>{formatHours(v.progress.used_secs)} of {formatHours(v.progress.goal_secs)}</span>
            <span class="state">
              {#if v.progress.state === 'over'}Over the limit
              {:else if v.progress.state === 'near'}Almost there
              {:else if v.progress.state === 'met'}Reached
              {:else if v.progress.state === 'behind'}Keep going
              {:else}Plenty of room{/if}
            </span>
          </div>
        </div>
        <div class="tools">
          <Switch checked={v.goal.enabled} label={`Goal for ${scopeLabel(v.goal.scope)} is on`} onchange={(on) => toggle(v.goal, on)} />
          <button class="icon" aria-label="Edit goal" title="Edit" onclick={() => edit(v.goal)}><Icon name="sliders" size={15} /></button>
          <button class="icon" aria-label="Delete goal" title="Delete" onclick={() => remove(v.goal.id)}><Icon name="trash" size={15} /></button>
        </div>
      </li>
    {/each}
  </ul>

  <div class="report">
    <div class="rhead">
      <h3>Weekly report</h3>
      <div class="seg" role="tablist" aria-label="Week">
        <button role="tab" aria-selected={showWeek === 'this'} class:on={showWeek === 'this'} onclick={() => (showWeek = 'this')}>This week</button>
        <button role="tab" aria-selected={showWeek === 'last'} class:on={showWeek === 'last'} onclick={() => (showWeek = 'last')}>Last week</button>
      </div>
    </div>
    {#if week && week.total_secs > 0}
      <div class="summary tabular">
        <strong>{formatHours(week.total_secs)}</strong>
        {#if sinceLastWeek(week)}<span class="delta" class:up={(changePercent(week) ?? 0) > 0}>{sinceLastWeek(week)}</span>{/if}
      </div>
      <div class="days" aria-label="Time on each day">
        {#each week.days as secs, i (i)}
          <div class="day" title={`${dayName(i)}: ${formatHours(secs)}`}>
            <i style:height="{Math.max(4, (secs / maxDay) * 100)}%"></i>
            <span>{dayName(i).slice(0, 1)}</span>
          </div>
        {/each}
      </div>
      {#if week.top_apps.length}
        <ol class="tops">
          {#each week.top_apps.slice(0, 3) as t (t.name)}
            <li><span translate="no">{t.name}</span><span class="tabular">{formatHours(t.secs)}</span></li>
          {/each}
        </ol>
      {/if}
      {#if week.goals.length}
        <ul class="results">
          {#each week.goals as r (r.scope + r.kind + r.period)}
            <li class:ok={r.kept === r.out_of}><Icon name={r.kept === r.out_of ? 'check' : 'info'} size={14} /> <span translate="no">{r.name}</span>: kept {r.kept} of {r.out_of} {r.out_of === 1 ? 'week' : 'days'}</li>
          {/each}
        </ul>
      {/if}
    {:else}
      <p class="empty">{showWeek === 'this' ? 'Nothing recorded this week yet.' : 'No report for last week: NomNom saw no activity.'}</p>
    {/if}
  </div>
</section>

<style>
  .goals { margin: 0 0 14px; padding: 16px 18px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border); box-shadow: var(--shadow-card); }
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 16px; }
  h2 { margin: 0 0 2px; font-size: 15.5px; }
  h3 { margin: 0; font-size: 13.5px; }
  .sub, .empty { margin: 0; color: var(--text-3); font-size: 12.5px; }
  .empty { padding: 10px 0 4px; }
  .err { color: var(--danger); margin: 8px 0 0; }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 6px 13px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; font-size: 13px; }
  button.plain:hover { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  button.primary:hover { background: var(--accent-hover); }
  .editor { margin: 14px 0 6px; padding: 14px; border-radius: var(--radius-m); background: var(--surface-2); border: 1px solid var(--border); display: grid; gap: 12px; }
  .kinds { display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  .kinds button { display: grid; gap: 2px; text-align: left; padding: 10px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-1); color: inherit; cursor: pointer; }
  .kinds button span { color: var(--text-3); font-size: 12px; }
  .kinds button.on { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .fields { display: flex; flex-wrap: wrap; gap: 12px; align-items: flex-end; }
  label { display: grid; gap: 4px; font-size: 12px; color: var(--text-3); font-weight: 600; }
  select, input[type='number'] { background: var(--surface-1); color: var(--text); border: 1px solid var(--border-strong); border-radius: var(--radius-xs); padding: 7px 9px; font: inherit; }
  input[type='number'] { width: 64px; }
  .pair { display: inline-flex; align-items: center; gap: 6px; color: var(--text-2); }
  .actions { display: flex; justify-content: flex-end; gap: 8px; }
  .list { list-style: none; margin: 12px 0 0; padding: 0; display: grid; gap: 10px; }
  .goal { --tone: var(--ok); display: flex; align-items: center; gap: 12px; padding: 11px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .goal.near { --tone: #ffc15c; }
  .goal.over { --tone: var(--danger); }
  .goal.behind { --tone: var(--info); }
  .goal.met { --tone: var(--ok); }
  .goal.off { opacity: 0.55; }
  .ico { flex: none; width: 36px; height: 36px; border-radius: 11px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); overflow: hidden; }
  .ico img { width: 26px; height: 26px; object-fit: contain; }
  .body { flex: 1; min-width: 0; display: grid; gap: 5px; }
  .row1, .row2 { display: flex; justify-content: space-between; gap: 10px; align-items: baseline; }
  .what { color: var(--text-3); font-size: 12px; }
  .row2 { font-size: 12px; color: var(--text-2); }
  .state { color: var(--tone); font-weight: 700; }
  .bar { height: 7px; border-radius: 99px; background: var(--surface-3); overflow: hidden; }
  .bar i { display: block; height: 100%; border-radius: 99px; background: var(--tone); transition: width var(--dur-3) var(--ease-out); }
  .tools { display: flex; align-items: center; gap: 6px; flex: none; }
  .icon { border: 0; background: none; color: var(--text-3); cursor: pointer; padding: 6px; border-radius: 8px; display: grid; }
  .icon:hover { background: var(--surface-3); color: var(--text); }
  .report { margin-top: 18px; padding-top: 14px; border-top: 1px solid var(--border); display: grid; gap: 10px; }
  .rhead { display: flex; justify-content: space-between; align-items: center; }
  .seg { display: inline-flex; padding: 3px; border-radius: 99px; background: var(--surface-2); border: 1px solid var(--border); }
  .seg button { border: 0; background: none; color: var(--text-2); padding: 4px 12px; border-radius: 99px; cursor: pointer; font-weight: 600; font-size: 12.5px; }
  .seg button.on { background: var(--surface-3); color: var(--text); }
  .summary { display: flex; align-items: baseline; gap: 10px; }
  .summary strong { font-family: var(--font-display); font-size: 24px; }
  .delta { font-size: 12.5px; color: var(--text-3); }
  .delta.up { color: var(--ok); }
  .days { display: grid; grid-template-columns: repeat(7, 1fr); gap: 8px; height: 76px; align-items: end; }
  .day { display: grid; grid-template-rows: 1fr auto; gap: 4px; height: 100%; text-align: center; color: var(--text-3); font-size: 11px; }
  .day i { align-self: end; width: 100%; border-radius: 5px 5px 2px 2px; background: var(--accent); opacity: 0.85; }
  .tops, .results { list-style: none; margin: 0; padding: 0; display: grid; gap: 4px; font-size: 13px; }
  .tops li { display: flex; justify-content: space-between; color: var(--text-2); }
  .results li { display: flex; align-items: center; gap: 7px; color: var(--text-3); }
  .results li.ok { color: var(--ok); }
</style>
