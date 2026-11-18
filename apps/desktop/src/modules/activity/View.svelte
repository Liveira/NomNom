<script lang="ts">
  import { every } from '../../lib/poll';
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import HeroEmblem from '../../lib/ui/HeroEmblem.svelte';
  import Switch from '../../lib/ui/Switch.svelte';
  import {
    actIpc,
    onActivityUpdated,
    type ActOverview,
    type ActRange,
  } from '../../lib/ipc';
  import AppBadge from './AppBadge.svelte';
  import AppDetailModal from './AppDetailModal.svelte';
  import FocusCard from './FocusCard.svelte';
  import {
    CATEGORIES,
    categoryInfo,
    formatDecimalHours,
    formatHourSlot,
    formatHours,
    formatSessionSpan,
    heatmapLevel,
    RANGES,
    relativeAgo,
  } from './logic';

  let range = $state<ActRange>('week');
  let categoryFilter = $state<string>('all');
  let query = $state('');
  let showHidden = $state(false);
  let data = $state<ActOverview | null>(null);
  let selectedAppId = $state<string | null>(null);
  let error = $state('');

  async function refresh() {
    try {
      data = await actIpc.overview(range, showHidden);
      error = '';
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    void range;
    void showHidden;
    void refresh();
  });

  $effect(() => {
    const unlisten = onActivityUpdated(() => void refresh());
    const poll = every(5000, () => void refresh());
    return () => {
      poll();
      void unlisten.then((fn) => fn());
    };
  });

  const enabled = $derived(app.settings?.activity.enabled ?? data?.enabled ?? true);
  const topApp = $derived(data?.apps[0] ?? null);
  const filteredApps = $derived(
    (data?.apps ?? []).filter((a) => {
      if (categoryFilter !== 'all' && a.category !== categoryFilter) return false;
      if (query.trim() && !a.name.toLowerCase().includes(query.trim().toLowerCase())) return false;
      return true;
    }),
  );

  const maxTimeline = $derived(Math.max(1, ...(data?.timeline.map((t) => t.secs) ?? [1])));
  const maxClock = $derived(Math.max(1, ...(data?.clock_hours ?? [1])));
  const maxAppSecs = $derived(Math.max(1, ...(filteredApps.map((a) => a.range_secs) ?? [1])));
</script>

<header class="studio-hero">
  <div>
    <div class="eyebrow">
      <Icon name="playtime" size={14} />
      <span>Active focus & game time</span>
    </div>
    <h1>Playtime</h1>
    <p>
      Counts only the app or game you have actively focused while using your computer. Background
      processes, minimized windows and idle breaks are never counted.
    </p>
    <div class="hero-bar">
      {#if !enabled}
        <span class="live-pill paused"><Icon name="pause" size={13} /> Tracking paused</span>
      {:else if data?.live}
        <span class="live-pill active">
          <span class="pulse"></span>
          Focused: <strong translate="no">{data.live.name}</strong> · <span class="tabular">{formatHours(data.live.session_secs)}</span>
        </span>
      {:else}
        <span class="live-pill"><span class="pulse calm"></span> Watching active foreground window</span>
      {/if}

      <div class="toggle-wrap">
        <span>Track active hours</span>
        <Switch
          checked={enabled}
          label="Track active hours"
          onchange={(v) => void app.update({ activityEnabled: v }).then(() => refresh())}
        />
      </div>
    </div>
  </div>
  <HeroEmblem icon="playtime" orbit={["flame", "clock", "star"]} size={104} />
</header>

<FocusCard />

<div class="toolbar">
  <div class="range-tabs" role="tablist" aria-label="Time period">
    {#each RANGES as r (r.value)}
      <button
        role="tab"
        aria-selected={range === r.value}
        class:on={range === r.value}
        onclick={() => (range = r.value)}
      >
        {r.label}
      </button>
    {/each}
  </div>

  {#if data}
    <div class="period-strip tabular">
      <span>Today <strong>{formatHours(data.today_secs)}</strong></span>
      <span>7d <strong>{formatHours(data.week_secs)}</strong></span>
      <span>30d <strong>{formatHours(data.month_secs)}</strong></span>
      <span>Year <strong>{formatHours(data.year_secs)}</strong></span>
    </div>
  {/if}
</div>

{#if error}
  <p class="err" role="alert">{error}</p>
{/if}

{#if data}
  <!-- Bento KPI Tiles -->
  <section class="bento-kpis" aria-label="Summary statistics">
    <div class="kpi-card highlight">
      <div class="k-head">
        <span class="k-title">Active Time</span>
        <Icon name="clock" size={16} />
      </div>
      <div class="k-val tabular">{formatHours(data.range_secs)}</div>
      <div class="k-sub tabular">All recorded time: {formatHours(data.all_time_secs)}</div>
    </div>

    <div class="kpi-card">
      <div class="k-head">
        <span class="k-title">Daily Rhythm</span>
        <Icon name="flame" size={16} />
      </div>
      <div class="k-val tabular">{formatHours(data.daily_avg_secs)}<span class="unit">/day</span></div>
      <div class="k-sub tabular">{data.sessions_count} active {data.sessions_count === 1 ? 'session' : 'sessions'}</div>
    </div>

    <div class="kpi-card">
      <div class="k-head">
        <span class="k-title">Top App</span>
        <Icon name="sparkle" size={16} />
      </div>
      {#if topApp}
        <div class="top-mini">
          <AppBadge name={topApp.name} category={topApp.category} iconUrl={topApp.icon_url} size={34} />
          <div class="top-text">
            <div class="top-name" translate="no" title={topApp.name}>{topApp.name}</div>
            <div class="k-sub tabular">{formatHours(topApp.range_secs)} · {Math.round(topApp.share * 100)}%</div>
          </div>
        </div>
      {:else}
        <div class="k-val muted-val">—</div>
        <div class="k-sub">Use any app to start</div>
      {/if}
    </div>

    <div class="kpi-card">
      <div class="k-head">
        <span class="k-title">Peak Window</span>
        <Icon name="calendar" size={16} />
      </div>
      <div class="k-val peak tabular">{formatHourSlot(data.peak_hour)}</div>
      <div class="k-sub">
        {#if data.categories[0]}
          Mostly {categoryInfo(data.categories[0].category).label} ({formatDecimalHours(data.categories[0].secs)})
        {:else}
          Idle cutoff: {Math.round(data.idle_timeout_secs / 60)} min
        {/if}
      </div>
    </div>
  </section>

  <!-- Interactive Charts Row -->
  <section class="charts-grid">
    <div class="panel">
      <div class="panel-head">
        <div>
          <h2>Activity timeline</h2>
          <p class="sub">
            {range === 'today'
              ? 'Active time by hour today'
              : range === 'week'
                ? 'Daily active hours across the last 7 days'
                : range === 'month'
                  ? 'Daily active hours across the last 30 days'
                  : 'Monthly active hours over the past year'}
          </p>
        </div>
      </div>

      <div class="timeline-bars" role="img" aria-label="Activity timeline chart">
        {#each data.timeline as pt, idx (pt.key)}
          {@const pct = pt.secs > 0 ? Math.max(7, Math.round((pt.secs / maxTimeline) * 100)) : 0}
          <div class="t-col" title={`${pt.label}: ${formatHours(pt.secs)}`}>
            <div class="t-track">
              <div class="t-bar" style:height="{pct}%"></div>
            </div>
            {#if data.timeline.length <= 12 || idx % (data.timeline.length > 24 ? 5 : 4) === 0 || idx === data.timeline.length - 1}
              <span class="t-label">{pt.label}</span>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <div class="panel side-panel">
      <div>
        <h2>By category</h2>
        {#if data.categories.length === 0}
          <p class="sub">Categories appear automatically as apps are used.</p>
        {:else}
          <ul class="cat-list">
            {#each data.categories as c (c.category)}
              {@const info = categoryInfo(c.category)}
              {@const pct = data.range_secs > 0 ? Math.round((c.secs / data.range_secs) * 100) : 0}
              <li>
                <div class="cat-line">
                  <span class="cat-dot" style:background={info.color}></span>
                  <span class="cat-name">{info.label}</span>
                  <span class="cat-val tabular">{formatHours(c.secs)} · {pct}%</span>
                </div>
                <div class="mini-track">
                  <div class="mini-fill" style:width="{Math.max(4, pct)}%" style:background={info.color}></div>
                </div>
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="clock-block">
        <h3>24-hour rhythm</h3>
        <div class="clock-bars">
          {#each data.clock_hours as secs, h (h)}
            <div class="c-col" title={`${String(h).padStart(2, '0')}:00 — ${formatHours(secs)}`}>
              <div class="c-track">
                <div class="c-bar" style:height="{secs > 0 ? Math.max(12, Math.round((secs / maxClock) * 100)) : 0}%"></div>
              </div>
              {#if h === 0 || h === 6 || h === 12 || h === 18 || h === 23}
                <span>{h}h</span>
              {/if}
            </div>
          {/each}
        </div>
      </div>
    </div>
  </section>

  <!-- 26-Week Activity Heatmap -->
  <section class="panel heatmap-panel">
    <div class="panel-head">
      <div>
        <h2>6-month activity map</h2>
        <p class="sub">Each square is one day of active foreground usage.</p>
      </div>
      <div class="legend" aria-hidden="true">
        <span>Less</span>
        <i class="lv-0"></i>
        <i class="lv-1"></i>
        <i class="lv-2"></i>
        <i class="lv-3"></i>
        <i class="lv-4"></i>
        <span>5h+</span>
      </div>
    </div>
    <div class="heat-grid">
      {#each data.heatmap as cell (cell.day)}
        <div
          class={`heat-cell lv-${heatmapLevel(cell.secs)}`}
          title={`${cell.day}: ${formatHours(cell.secs)}`}
        ></div>
      {/each}
    </div>
  </section>

  <!-- Apps Showcase + Recent Sessions -->
  <section class="bottom-grid">
    <div class="panel apps-panel">
      <div class="apps-toolbar">
        <h2>Tracked apps & games</h2>
        <div class="search-box">
          <Icon name="search" size={15} />
          <input type="text" placeholder="Filter apps…" aria-label="Filter apps" bind:value={query} />
        </div>
      </div>

      <div class="cat-pills">
        <button class:on={categoryFilter === 'all'} onclick={() => (categoryFilter = 'all')}>All</button>
        {#each CATEGORIES as c (c.id)}
          <button class:on={categoryFilter === c.id} onclick={() => (categoryFilter = c.id)}>{c.label}</button>
        {/each}
        <button class="ghost-pill" class:on={showHidden} onclick={() => (showHidden = !showHidden)}>
          <Icon name={showHidden ? 'eye' : 'eye-off'} size={13} /> Hidden
        </button>
      </div>

      {#if filteredApps.length === 0}
        <div class="empty-box">
          <CatNap width={104} />
          <div>
            <p class="empty-title">No active hours in this view yet</p>
            <p class="sub">
              Switch to Chrome, Discord, Clip Studio Paint or any game — NomNom picks up the active
              window automatically within a couple of seconds.
            </p>
          </div>
        </div>
      {:else}
        <ul class="app-list">
          {#each filteredApps as item (item.id)}
            {@const c = categoryInfo(item.category)}
            {@const barPct = Math.max(3, Math.round((item.range_secs / maxAppSecs) * 100))}
            <li>
              <button class="app-row" onclick={() => (selectedAppId = item.id)}>
                <AppBadge name={item.name} category={item.category} iconUrl={item.icon_url} size={42} />
                <div class="app-main">
                  <div class="app-line">
                    <span class="app-name">
                      {item.name}
                      {#if item.hidden}<span class="muted-tag">hidden</span>{/if}
                    </span>
                    <strong class="app-hours tabular">{formatHours(item.range_secs)}</strong>
                  </div>
                  <div class="app-bar-track">
                    <div class="app-bar-fill" style:width="{barPct}%" style:background={c.color}></div>
                  </div>
                  <div class="app-meta tabular">
                    <span class="cat-chip" style:color={c.color} style:background={c.tint}>{c.label}</span>
                    <span>Last used {relativeAgo(item.last_seen)}</span>
                    <span>·</span>
                    <span>7d {formatDecimalHours(item.week_secs)}</span>
                    <span>·</span>
                    <span>30d {formatDecimalHours(item.month_secs)}</span>
                    <span>·</span>
                    <span>Year {formatDecimalHours(item.year_secs)}</span>
                  </div>
                </div>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>

    <div class="panel sessions-panel">
      <div class="panel-head">
        <div>
          <h2>When you used them</h2>
          <p class="sub">Latest completed foreground sessions.</p>
        </div>
      </div>

      {#if data.recent_sessions.length === 0}
        <p class="sub empty-sessions">Completed sessions appear here as you switch between apps.</p>
      {:else}
        <ul class="session-list">
          {#each data.recent_sessions as s (s.id)}
            <li>
              <button class="session-row" onclick={() => (selectedAppId = s.app_id)}>
                <AppBadge name={s.app_name} category={s.category} iconUrl={s.icon_url} size={32} />
                <div class="s-body">
                  <div class="s-name" translate="no">{s.app_name}</div>
                  <div class="s-when">{formatSessionSpan(s.started_at, s.ended_at)}</div>
                </div>
                <strong class="s-dur tabular">{formatHours(s.active_secs)}</strong>
              </button>
            </li>
          {/each}
        </ul>
      {/if}
    </div>
  </section>
{/if}

{#if selectedAppId}
  <AppDetailModal
    appId={selectedAppId}
    onclose={() => (selectedAppId = null)}
    onchanged={() => void refresh()}
  />
{/if}

<style>
  .hero-bar {
    display: flex;
    align-items: center;
    gap: 14px;
    flex-wrap: wrap;
    margin-top: 12px;
  }
  .live-pill {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    padding: 5px 12px;
    border-radius: var(--radius-pill);
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    font-size: 12.5px;
    color: var(--text-2);
  }
  .live-pill.active {
    border-color: rgba(123, 232, 188, 0.4);
    color: var(--text);
  }
  .live-pill.paused { color: var(--text-3); }
  .pulse {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--ok);
    box-shadow: 0 0 10px var(--ok);
  }
  .pulse.calm { background: var(--accent); box-shadow: none; }
  .toggle-wrap {
    display: inline-flex;
    align-items: center;
    gap: 8px;
    font-size: 12.5px;
    color: var(--text-2);
  }

  .toolbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 14px;
    flex-wrap: wrap;
    margin-bottom: 16px;
  }
  .range-tabs {
    display: inline-flex;
    padding: 4px;
    gap: 2px;
    border-radius: var(--radius-pill);
    background: var(--surface-1);
    border: 1px solid var(--border-strong);
    box-shadow: var(--shadow-soft);
  }
  .range-tabs button {
    border: 0;
    background: transparent;
    color: var(--text-2);
    padding: 6px 14px;
    border-radius: var(--radius-pill);
    font-weight: 600;
    font-size: 13px;
    cursor: pointer;
    transition: background var(--dur-1), color var(--dur-1);
  }
  .range-tabs button:hover { color: var(--text); }
  .range-tabs button.on {
    background: linear-gradient(135deg, var(--accent), var(--blush));
    color: var(--on-accent);
  }
  .period-strip {
    display: flex;
    gap: 14px;
    flex-wrap: wrap;
    font-size: 12.5px;
    color: var(--text-3);
  }
  .period-strip strong { color: var(--text); font-weight: 700; }

  .bento-kpis {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(195px, 1fr));
    gap: 12px;
    margin-bottom: 14px;
  }
  .kpi-card {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 14px 16px;
    box-shadow: var(--shadow-card);
    display: grid;
    gap: 6px;
  }
  .kpi-card.highlight {
    background:
      radial-gradient(180px 120px at 100% 0%, var(--glow-peach), transparent 75%),
      var(--surface-1);
    border-color: var(--border-strong);
  }
  .k-head { display: flex; justify-content: space-between; align-items: center; color: var(--text-3); }
  .k-title { font-size: 12px; font-weight: 700; letter-spacing: 0.02em; }
  .k-val { font-family: var(--font-display); font-size: 25px; font-weight: 700; line-height: 1.15; }
  .k-val.peak { font-size: 20px; }
  .muted-val { color: var(--text-3); }
  .unit { font-size: 13px; color: var(--text-3); font-weight: 500; margin-left: 2px; }
  .k-sub { font-size: 12px; color: var(--text-3); }
  .top-mini { display: flex; align-items: center; gap: 10px; min-width: 0; }
  .top-text { min-width: 0; }
  .top-name { font-family: var(--font-display); font-weight: 700; font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

  .charts-grid {
    display: grid;
    grid-template-columns: 1.55fr 1fr;
    gap: 14px;
    margin-bottom: 14px;
  }
  @media (max-width: 920px) {
    .charts-grid, .bottom-grid { grid-template-columns: 1fr; }
  }
  .panel {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    padding: 16px 18px;
    box-shadow: var(--shadow-card);
  }
  .panel-head { display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; margin-bottom: 12px; }
  .sub { margin: 2px 0 0; color: var(--text-3); font-size: 12.5px; }

  .timeline-bars {
    height: 168px;
    display: grid;
    grid-auto-flow: column;
    grid-auto-columns: 1fr;
    gap: 5px;
    align-items: end;
    padding-top: 8px;
  }
  .t-col { height: 100%; display: flex; flex-direction: column; align-items: center; gap: 5px; min-width: 0; }
  .t-track {
    flex: 1;
    width: 100%;
    border-radius: 8px;
    background: var(--surface-2);
    display: flex;
    align-items: flex-end;
    overflow: hidden;
  }
  .t-bar {
    width: 100%;
    border-radius: 8px 8px 3px 3px;
    background: linear-gradient(180deg, var(--accent) 0%, var(--blush) 100%);
    transition: height var(--dur-3) var(--ease-out);
  }
  .t-col:hover .t-bar { filter: brightness(1.12); }
  .t-label { font-size: 10.5px; color: var(--text-3); white-space: nowrap; }

  .side-panel { display: grid; gap: 16px; align-content: space-between; }
  .cat-list { list-style: none; margin: 10px 0 0; padding: 0; display: grid; gap: 9px; }
  .cat-line { display: flex; align-items: center; gap: 8px; font-size: 12.5px; margin-bottom: 4px; }
  .cat-dot { width: 8px; height: 8px; border-radius: 50%; flex: none; }
  .cat-name { font-weight: 600; }
  .cat-val { margin-left: auto; color: var(--text-2); }
  .mini-track { height: 5px; border-radius: var(--radius-pill); background: var(--surface-2); overflow: hidden; }
  .mini-fill { height: 100%; border-radius: var(--radius-pill); }

  .clock-block h3 { font-size: 13px; color: var(--text-2); margin-bottom: 8px; }
  .clock-bars { height: 56px; display: grid; grid-template-columns: repeat(24, 1fr); gap: 3px; align-items: end; }
  .c-col { height: 100%; display: flex; flex-direction: column; align-items: center; gap: 3px; }
  .c-track { flex: 1; width: 100%; background: var(--surface-2); border-radius: 4px; display: flex; align-items: flex-end; overflow: hidden; }
  .c-bar { width: 100%; background: var(--lavender); border-radius: 4px; }
  .c-col span { font-size: 9.5px; color: var(--text-3); }

  .heatmap-panel { margin-bottom: 14px; }
  .heat-grid {
    display: grid;
    grid-template-rows: repeat(7, 12px);
    grid-auto-flow: column;
    grid-auto-columns: 12px;
    gap: 4px;
    overflow-x: auto;
    padding-bottom: 4px;
  }
  .heat-cell, .legend i {
    width: 12px;
    height: 12px;
    border-radius: 3.5px;
    display: inline-block;
  }
  .lv-0 { background: var(--surface-2); }
  .lv-1 { background: rgba(192, 163, 255, 0.28); }
  .lv-2 { background: rgba(255, 143, 184, 0.48); }
  .lv-3 { background: rgba(255, 169, 112, 0.72); }
  .lv-4 { background: var(--accent); box-shadow: 0 0 8px var(--glow-peach); }
  .legend { display: flex; align-items: center; gap: 4px; font-size: 11.5px; color: var(--text-3); }

  .bottom-grid {
    display: grid;
    grid-template-columns: 1.55fr 1fr;
    gap: 14px;
    align-items: start;
  }
  .apps-toolbar { display: flex; justify-content: space-between; align-items: center; gap: 12px; flex-wrap: wrap; margin-bottom: 10px; }
  .search-box {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 6px 12px;
    border-radius: var(--radius-pill);
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    color: var(--text-3);
  }
  .search-box input { border: 0; background: none; outline: none; color: var(--text); width: 160px; font-size: 12.5px; }
  .cat-pills { display: flex; gap: 6px; flex-wrap: wrap; margin-bottom: 14px; }
  .cat-pills button {
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    padding: 4px 11px;
    border-radius: var(--radius-pill);
    font-size: 12px;
    cursor: pointer;
    display: inline-flex;
    align-items: center;
    gap: 5px;
  }
  .cat-pills button.on {
    background: var(--surface-3);
    border-color: var(--border-glow);
    color: var(--accent-text);
    font-weight: 700;
  }
  .ghost-pill { margin-left: auto; }

  .empty-box { display: flex; align-items: center; gap: 16px; padding: 18px 8px; }
  .empty-title { margin: 0; font-family: var(--font-display); font-weight: 700; font-size: 15px; }

  .app-list, .session-list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; }
  .app-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 13px;
    padding: 11px 13px;
    border-radius: var(--radius-m);
    background: var(--surface-2);
    border: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    transition: background var(--dur-1), border-color var(--dur-1), transform var(--dur-1);
  }
  .app-row:hover {
    background: var(--surface-3);
    border-color: var(--border-strong);
  }
  .app-main { flex: 1; min-width: 0; display: grid; gap: 6px; }
  .app-line { display: flex; justify-content: space-between; align-items: baseline; gap: 10px; }
  .app-name { font-family: var(--font-display); font-weight: 700; font-size: 14.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .muted-tag { font-size: 11px; color: var(--text-3); margin-left: 6px; font-weight: 500; }
  .app-hours { font-family: var(--font-display); font-size: 15px; color: var(--accent-text); flex: none; }
  .app-bar-track { height: 5px; border-radius: var(--radius-pill); background: var(--surface-1); overflow: hidden; }
  .app-bar-fill { height: 100%; border-radius: var(--radius-pill); transition: width var(--dur-3) var(--ease-out); }
  .app-meta { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; font-size: 11.5px; color: var(--text-3); }
  .cat-chip { padding: 1px 8px; border-radius: var(--radius-pill); font-weight: 700; font-size: 11px; }

  .empty-sessions { padding: 12px 0; }
  .session-row {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid transparent;
    cursor: pointer;
    text-align: left;
    transition: background var(--dur-1), border-color var(--dur-1);
  }
  .session-row:hover { background: var(--surface-3); border-color: var(--border-strong); }
  .s-body { flex: 1; min-width: 0; }
  .s-name { font-weight: 600; font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .s-when { font-size: 11.5px; color: var(--text-3); }
  .s-dur { font-family: var(--font-display); font-size: 13.5px; color: var(--text-2); flex: none; }
  .err { color: var(--danger); }
</style>
