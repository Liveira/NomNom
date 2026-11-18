<script lang="ts">
  import Dialog from '../../lib/ui/Dialog.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { actIpc, type ActAppDetail } from '../../lib/ipc';
  import AppBadge from './AppBadge.svelte';
  import {
    CATEGORIES,
    categoryInfo,
    formatHourSlot,
    formatHours,
    formatSessionSpan,
    relativeAgo,
  } from './logic';

  let {
    appId,
    onclose,
    onchanged,
  }: {
    appId: string;
    onclose: () => void;
    onchanged: () => void;
  } = $props();

  let detail = $state<ActAppDetail | null>(null);
  let error = $state('');
  let chartMode = $state<'30d' | '12m'>('30d');
  let customName = $state('');
  let category = $state('general');
  let saving = $state(false);
  let confirmDelete = $state(false);

  async function load() {
    error = '';
    try {
      const d = await actIpc.appDetail(appId);
      detail = d;
      customName = d.app.name;
      category = d.app.category;
    } catch (e) {
      error = String(e);
    }
  }

  $effect(() => {
    void appId;
    void load();
  });

  async function saveMeta() {
    if (!detail) return;
    saving = true;
    try {
      await actIpc.updateApp(detail.app.id, { customName, category });
      await load();
      onchanged();
    } catch (e) {
      error = String(e);
    } finally {
      saving = false;
    }
  }

  async function toggleHidden() {
    if (!detail) return;
    await actIpc.updateApp(detail.app.id, { hidden: !detail.app.hidden });
    await load();
    onchanged();
  }

  async function removeApp() {
    if (!detail) return;
    await actIpc.deleteApp(detail.app.id);
    onchanged();
    onclose();
  }

  const series = $derived(detail ? (chartMode === '30d' ? detail.daily_30d : detail.monthly_12m) : []);
  const maxSeries = $derived(Math.max(1, ...series.map((p) => p.secs)));
  const maxClock = $derived(detail ? Math.max(1, ...detail.clock_hours) : 1);
  const cat = $derived(categoryInfo(detail?.app.category ?? 'general'));
</script>

<Dialog title={detail ? detail.app.name : 'App activity'} {onclose}>
  {#if error}
    <p class="err" role="alert">{error}</p>
  {:else if !detail}
    <p class="muted">Loading activity history…</p>
  {:else}
    <div class="sheet">
      <div class="top">
        <AppBadge name={detail.app.name} category={detail.app.category} iconUrl={detail.app.icon_url} size={52} />
        <div class="meta">
          <div class="badges">
            <span class="pill" style:color={cat.color} style:background={cat.tint}>{cat.label}</span>
            <span class="muted">Last active {relativeAgo(detail.app.last_seen)}</span>
            {#if detail.app.hidden}<span class="pill hidden-pill">Hidden</span>{/if}
          </div>
          {#if detail.app.exe_path}
            <div class="path" translate="no" title={detail.app.exe_path}>{detail.app.exe_path}</div>
          {/if}
        </div>
      </div>

      <div class="kpis">
        <div class="kpi">
          <span class="k-label">Today</span>
          <strong class="tabular">{formatHours(detail.app.today_secs)}</strong>
        </div>
        <div class="kpi">
          <span class="k-label">7 Days</span>
          <strong class="tabular">{formatHours(detail.app.week_secs)}</strong>
        </div>
        <div class="kpi">
          <span class="k-label">30 Days</span>
          <strong class="tabular">{formatHours(detail.app.month_secs)}</strong>
        </div>
        <div class="kpi">
          <span class="k-label">This Year</span>
          <strong class="tabular">{formatHours(detail.app.year_secs)}</strong>
        </div>
        <div class="kpi">
          <span class="k-label">All Time</span>
          <strong class="tabular">{formatHours(detail.app.total_secs)}</strong>
        </div>
      </div>

      <div class="sub-stats">
        <span><strong>{detail.active_days}</strong> active {detail.active_days === 1 ? 'day' : 'days'}</span>
        <span>·</span>
        <span><strong>{detail.sessions_count}</strong> {detail.sessions_count === 1 ? 'session' : 'sessions'}</span>
        <span>·</span>
        <span>Longest <strong>{formatHours(detail.longest_session_secs)}</strong></span>
        <span>·</span>
        <span>Peak <strong>{formatHourSlot(detail.peak_hour)}</strong></span>
      </div>

      <div class="chart-card">
        <div class="chart-head">
          <h3>Usage history</h3>
          <div class="seg" role="tablist" aria-label="Chart span">
            <button role="tab" aria-selected={chartMode === '30d'} class:on={chartMode === '30d'} onclick={() => (chartMode = '30d')}>30 Days</button>
            <button role="tab" aria-selected={chartMode === '12m'} class:on={chartMode === '12m'} onclick={() => (chartMode = '12m')}>12 Months</button>
          </div>
        </div>
        <div class="bars" style:--bar-color={cat.color}>
          {#each series as pt (pt.key)}
            <div class="col" title={`${pt.label}: ${formatHours(pt.secs)}`}>
              <div class="track">
                <div class="bar" style:height="{pt.secs > 0 ? Math.max(8, Math.round((pt.secs / maxSeries) * 100)) : 0}%"></div>
              </div>
              {#if series.length <= 12 || pt.key.endsWith('01') || pt.key.endsWith('10') || pt.key.endsWith('20')}
                <span class="tick">{pt.label}</span>
              {/if}
            </div>
          {/each}
        </div>
      </div>

      <div class="chart-card">
        <h3>Time of day (00h – 23h)</h3>
        <div class="bars compact" style:--bar-color="var(--accent)">
          {#each detail.clock_hours as secs, h (h)}
            <div class="col" title={`${String(h).padStart(2, '0')}:00 — ${formatHours(secs)}`}>
              <div class="track">
                <div class="bar" style:height="{secs > 0 ? Math.max(10, Math.round((secs / maxClock) * 100)) : 0}%"></div>
              </div>
              {#if h % 6 === 0}<span class="tick">{String(h).padStart(2, '0')}h</span>{/if}
            </div>
          {/each}
        </div>
      </div>

      <div class="edit-row">
        <label>
          <span>Display name</span>
          <input type="text" bind:value={customName} />
        </label>
        <label>
          <span>Category</span>
          <select bind:value={category}>
            {#each CATEGORIES as c (c.id)}
              <option value={c.id}>{c.label}</option>
            {/each}
          </select>
        </label>
        <button class="save-btn" onclick={saveMeta} disabled={saving}>Save</button>
      </div>

      {#if detail.recent_sessions.length > 0}
        <div class="sessions">
          <h3>Recent sessions</h3>
          <ul>
            {#each detail.recent_sessions.slice(0, 15) as s (s.id)}
              <li>
                <span>{formatSessionSpan(s.started_at, s.ended_at)}</span>
                <strong class="tabular">{formatHours(s.active_secs)}</strong>
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      <div class="danger-row">
        <button class="ghost" onclick={toggleHidden}>
          <Icon name={detail.app.hidden ? 'eye' : 'eye-off'} size={15} />
          {detail.app.hidden ? 'Show in Playtime' : 'Hide from Playtime'}
        </button>
        {#if confirmDelete}
          <div class="confirm">
            <span>Remove all recorded hours?</span>
            <button class="danger" onclick={removeApp}>Yes, remove</button>
            <button class="ghost" onclick={() => (confirmDelete = false)}>Cancel</button>
          </div>
        {:else}
          <button class="ghost danger-text" onclick={() => (confirmDelete = true)}>
            <Icon name="trash" size={15} /> Clear history
          </button>
        {/if}
      </div>
    </div>
  {/if}

  {#snippet actions()}
    <button class="close-btn" onclick={onclose}>Done</button>
  {/snippet}
</Dialog>

<style>
  .sheet { display: grid; gap: 14px; padding-right: 4px; }
  .top { display: flex; align-items: center; gap: 14px; }
  .meta { min-width: 0; flex: 1; display: grid; gap: 4px; }
  .badges { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; }
  .pill { padding: 2px 10px; border-radius: var(--radius-pill); font-size: 11.5px; font-weight: 700; }
  .hidden-pill { background: var(--surface-3); color: var(--text-2); }
  .muted { color: var(--text-3); font-size: 12.5px; }
  .err { color: var(--danger); margin: 0; }
  .path { font-family: var(--font-mono); font-size: 11.5px; color: var(--text-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; user-select: text; }
  .kpis { display: grid; grid-template-columns: repeat(5, 1fr); gap: 8px; }
  .kpi { background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--radius-s); padding: 9px 10px; display: grid; gap: 2px; }
  .k-label { font-size: 11px; color: var(--text-3); }
  .kpi strong { font-family: var(--font-display); font-size: 15.5px; }
  .sub-stats { display: flex; flex-wrap: wrap; gap: 8px; font-size: 12.5px; color: var(--text-2); }
  .chart-card { background: var(--surface-2); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 12px 14px; display: grid; gap: 10px; }
  .chart-head { display: flex; justify-content: space-between; align-items: center; }
  h3 { font-size: 13.5px; }
  .seg { display: inline-flex; background: var(--surface-1); padding: 2px; border-radius: var(--radius-pill); border: 1px solid var(--border); }
  .seg button { border: 0; background: none; color: var(--text-3); padding: 3px 10px; border-radius: var(--radius-pill); font-size: 12px; cursor: pointer; }
  .seg button.on { background: var(--surface-3); color: var(--text); font-weight: 600; }
  .bars { display: grid; grid-auto-flow: column; grid-auto-columns: 1fr; gap: 4px; height: 92px; align-items: end; }
  .bars.compact { height: 64px; }
  .col { height: 100%; display: flex; flex-direction: column; align-items: center; gap: 4px; }
  .track { flex: 1; width: 100%; background: var(--surface-1); border-radius: 6px; display: flex; align-items: flex-end; overflow: hidden; }
  .bar { width: 100%; background: var(--bar-color); border-radius: 6px 6px 2px 2px; transition: height var(--dur-2) var(--ease-out); }
  .tick { font-size: 10px; color: var(--text-3); white-space: nowrap; }
  .edit-row { display: grid; grid-template-columns: 1fr 148px auto; gap: 8px; align-items: end; background: var(--surface-2); padding: 10px 12px; border-radius: var(--radius-m); border: 1px solid var(--border); }
  .edit-row label { display: grid; gap: 4px; font-size: 11.5px; color: var(--text-3); }
  input, select { padding: 7px 10px; border-radius: var(--radius-xs); border: 1px solid var(--border-strong); background: var(--surface-1); color: var(--text); }
  .save-btn, .close-btn { padding: 7px 16px; border-radius: var(--radius-s); border: 0; background: var(--accent); color: var(--on-accent); font-weight: 700; cursor: pointer; }
  .sessions ul { list-style: none; margin: 6px 0 0; padding: 0; max-height: 150px; overflow: auto; display: grid; gap: 4px; }
  .sessions li { display: flex; justify-content: space-between; padding: 6px 10px; border-radius: var(--radius-xs); background: var(--surface-2); font-size: 12.5px; color: var(--text-2); }
  .sessions li strong { color: var(--text); }
  .danger-row { display: flex; justify-content: space-between; align-items: center; gap: 10px; flex-wrap: wrap; padding-top: 4px; border-top: 1px solid var(--border); }
  .ghost { display: inline-flex; align-items: center; gap: 6px; border: 1px solid var(--border-strong); background: var(--surface-2); padding: 6px 12px; border-radius: var(--radius-s); cursor: pointer; font-size: 12.5px; }
  .danger-text { color: var(--danger); }
  .danger { border: 0; background: var(--danger); color: #24060b; font-weight: 700; padding: 6px 12px; border-radius: var(--radius-s); cursor: pointer; font-size: 12.5px; }
  .confirm { display: flex; align-items: center; gap: 8px; font-size: 12.5px; }
</style>
