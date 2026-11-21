<script lang="ts">
  import { every } from '../../lib/poll';
  import Icon from '../../lib/icons/Icon.svelte';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { TOOLS } from '../../lib/tools';
  import { app } from '../../app/state.svelte';
  import { progressLine } from '../../lib/format';
  import { actIpc, onActivityUpdated, type ActOverview } from '../../lib/ipc';
  import AppBadge from '../activity/AppBadge.svelte';
  import { categoryInfo, formatHours } from '../activity/logic';
  import { downloads } from '../downloads/store.svelte';
  import FirstRun from './FirstRun.svelte';
  import Glance from './Glance.svelte';

  let play = $state<ActOverview | null>(null);

  async function loadPlay() {
    try {
      play = await actIpc.overview('today', false);
    } catch {
      /* activity overview optional on initial mount */
    }
  }

  $effect(() => {
    void loadPlay();
    const unlisten = onActivityUpdated(() => void loadPlay());
    const timer = every(6000, () => void loadPlay());
    return () => {
      timer();
      void unlisten.then((fn) => fn());
    };
  });

  const active = $derived(downloads.active.slice(0, 3));
  const more = $derived(Math.max(0, downloads.active.length - 3));
  const topApps = $derived((play?.apps ?? []).slice(0, 3));
  const maxTodayApp = $derived(Math.max(1, ...(topApps.map((a) => a.range_secs) ?? [1])));

  const tools = $derived(TOOLS.filter((t) => app.sidebarNav.some((n) => n.id === t.id)));
</script>

<header class="studio-hero">
  <div>
    <h1>Welcome back to NomNom</h1>
    <p>
      Downloads, active app & game hours, file conversions, folder sorting and disk space analysis —
      all private on your computer.
    </p>
  </div>
</header>

{#if app.settings && !app.settings.system.first_run_done}
  <FirstRun />
{/if}

<Glance />

<section class="bento-main">
  <!-- Playtime Bento Card -->
  <div class="bento-card">
    <div class="head">
      <div>
        <h3>Today's Playtime</h3>
        <span class="sub tabular">
          {#if play}
            {formatHours(play.today_secs)} today · {formatHours(play.week_secs)} this week
          {:else}
            Active foreground apps & games
          {/if}
        </span>
      </div>
      <button class="pill-link" onclick={() => app.go('activity')}>
        Open Playtime <Icon name="arrow" size={14} />
      </button>
    </div>

    {#if topApps.length === 0}
      <div class="nap-row">
        <CatNap width={76} />
        <p class="empty">
          Active app & game hours show up automatically as you work, draw or play.
        </p>
      </div>
    {:else}
      <ul class="mini-apps">
        {#each topApps as item (item.id)}
          {@const c = categoryInfo(item.category)}
          <li>
            <AppBadge name={item.name} category={item.category} iconUrl={item.icon_url} size={32} />
            <div class="m-body">
              <div class="m-line">
                <span class="m-name" translate="no">{item.name}</span>
                <strong class="tabular">{formatHours(item.range_secs)}</strong>
              </div>
              <div class="m-track">
                <div
                  class="m-fill"
                  style:width="{Math.max(6, Math.round((item.range_secs / maxTodayApp) * 100))}%"
                  style:background={c.color}
                ></div>
              </div>
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  </div>

  <!-- Active Downloads Bento Card -->
  <div class="bento-card">
    <div class="head">
      <div>
        <h3>Active downloads</h3>
        <span class="sub">Queue & quick shortcut (`Ctrl+Alt+D`)</span>
      </div>
      <button class="pill-link" onclick={() => app.go('downloads')}>
        Open Downloader <Icon name="arrow" size={14} />
      </button>
    </div>

    {#if active.length === 0}
      <div class="nap-row">
        <CatNap width={76} />
        <p class="empty">Nothing downloading. Paste a link in NomNom Downloader to wake the cat.</p>
      </div>
    {:else}
      <ul class="dl-list">
        {#each active as job (job.id)}
          {@const live = downloads.progressFor(job.id)}
          <li>
            <div class="line">
              <span class="title" translate="no">{job.title}</span>
              <span class="state tabular">
                {#if job.status === 'paused'}Paused
                {:else if live}{progressLine(live)}
                {:else}Waiting{/if}
              </span>
            </div>
            {#if job.status === 'running'}
              <ProgressBar value={live?.fraction ?? null} label={`Progress for ${job.title}`} />
            {/if}
          </li>
        {/each}
      </ul>
      {#if more > 0}<p class="empty">and {more} more</p>{/if}
    {/if}
  </div>
</section>

<h2 class="grid-title">Your toolkit</h2>
<section class="tool-grid">
  {#each tools as t (t.id)}
    <button class="tool-tile" onclick={() => app.go(t.id)}>
      <div class="tile-icon">
        <Icon name={t.icon} size={18} />
      </div>
      <div class="tile-copy">
        <div class="tile-title">{t.title}</div>
        <p class="tile-desc">{t.desc}</p>
      </div>
    </button>
  {/each}
  <button class="tool-tile muted" onclick={() => app.go('tools')}>
    <div class="tile-icon"><Icon name="tools" size={18} /></div>
    <div class="tile-copy">
      <div class="tile-title">Add or remove tools</div>
      <p class="tile-desc">Choose which tools show in the sidebar.</p>
    </div>
  </button>
</section>

<style>
  .bento-main {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
    gap: 14px;
    margin-bottom: 24px;
  }
  .bento-card {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-m);
    padding: 16px 18px;
    box-shadow: var(--shadow-card);
    display: grid;
    align-content: start;
    gap: 14px;
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .sub {
    display: block;
    font-size: 12px;
    color: var(--text-3);
    margin-top: 2px;
  }
  .pill-link {
    display: inline-flex;
    align-items: center;
    gap: 4px;
    padding: 4px 10px;
    border-radius: var(--radius-s);
    border: 1px solid var(--border);
    background: var(--surface-2);
    color: var(--text-2);
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--dur-1), border-color var(--dur-1), color var(--dur-1);
  }
  .pill-link:hover {
    background: var(--surface-3);
    border-color: var(--border-strong);
    color: var(--text-1);
  }

  .nap-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 4px 0;
  }
  .empty {
    margin: 0;
    color: var(--text-3);
    font-size: 13px;
    line-height: 1.45;
  }

  .mini-apps, .dl-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 8px;
  }
  .mini-apps li {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px 10px;
    border-radius: var(--radius-s);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .m-body { flex: 1; min-width: 0; display: grid; gap: 5px; }
  .m-line { display: flex; justify-content: space-between; gap: 8px; font-size: 13px; }
  .m-name { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .m-track { height: 4px; border-radius: var(--radius-pill); background: var(--surface-1); overflow: hidden; }
  .m-fill { height: 100%; border-radius: var(--radius-pill); }

  .line { display: flex; justify-content: space-between; gap: 12px; margin-bottom: 6px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .state { color: var(--text-2); flex: none; font-size: 12px; }

  .grid-title {
    margin: 0 0 12px;
    font-size: 15px;
    color: var(--text-2);
    font-weight: 600;
  }
  .tool-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(230px, 1fr));
    gap: 10px;
  }
  .tool-tile {
    display: flex;
    align-items: flex-start;
    gap: 12px;
    padding: 13px 14px;
    border-radius: var(--radius-m);
    border: 1px solid var(--border);
    background: var(--surface-1);
    text-align: left;
    cursor: pointer;
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .tool-tile:hover {
    background: var(--surface-2);
    border-color: var(--border-strong);
  }
  .tool-tile.muted {
    border-style: dashed;
    background: transparent;
  }
  .tool-tile.muted:hover {
    background: var(--surface-1);
  }
  .tile-icon {
    width: 34px;
    height: 34px;
    flex: none;
    border-radius: var(--radius-s);
    display: grid;
    place-items: center;
    background: var(--surface-2);
    border: 1px solid var(--border);
    color: var(--accent-text);
  }
  .tile-copy {
    min-width: 0;
    display: grid;
    gap: 2px;
  }
  .tile-title {
    font-weight: 600;
    font-size: 14px;
    line-height: 1.3;
  }
  .tile-desc {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-3);
    line-height: 1.4;
  }
</style>
