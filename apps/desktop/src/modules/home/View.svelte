<script lang="ts">
  import { every } from '../../lib/poll';
  import Icon from '../../lib/icons/Icon.svelte';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import HeroEmblem from '../../lib/ui/HeroEmblem.svelte';
  import { app } from '../../app/state.svelte';
  import { progressLine } from '../../lib/format';
  import { actIpc, onActivityUpdated, type ActOverview } from '../../lib/ipc';
  import AppBadge from '../activity/AppBadge.svelte';
  import { categoryInfo, formatHours } from '../activity/logic';
  import { downloads } from '../downloads/store.svelte';
  import FirstRun from './FirstRun.svelte';

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

  const tools = [
    {
      id: 'downloads',
      icon: 'cat',
      title: 'NomNom Downloader',
      desc: 'Grab videos and music with queue, history and format choice.',
      tint: 'var(--glow-peach)',
    },
    {
      id: 'activity',
      icon: 'playtime',
      title: 'Playtime',
      desc: 'See active hours for your daily apps, creative tools and games.',
      tint: 'var(--glow-blush)',
    },
    {
      id: 'storage',
      icon: 'storage',
      title: 'Disk Analyzer',
      desc: 'Visual treemap, sunburst, duplicates and deep space cleanup.',
      tint: 'var(--glow-lavender)',
    },
    {
      id: 'filetools',
      icon: 'convert',
      title: 'File Tools',
      desc: 'Convert images, audio, video and Office documents locally.',
      tint: 'var(--glow-mint)',
    },
    {
      id: 'organizer',
      icon: 'organizer',
      title: 'File Organizer',
      desc: 'Preview-first folder sorting with full one-click undo.',
      tint: 'var(--glow-peach)',
    },
  ];
</script>

<header class="studio-hero">
  <div>
    <div class="eyebrow">
      <Icon name="sparkle" size={14} />
      <span>Local studio hub</span>
    </div>
    <h1>Welcome back to NomNom</h1>
    <p>
      Downloads, active app & game hours, file conversions, folder sorting and disk space analysis —
      all private on your computer.
    </p>
  </div>
  <HeroEmblem icon="cat" orbit={["sparkle", "heart", "star"]} size={104} />
</header>

{#if app.settings && !app.settings.system.first_run_done}
  <FirstRun />
{/if}

<section class="bento-main">
  <!-- Playtime Bento Card -->
  <div class="bento-card play-card">
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
        <CatNap width={96} />
        <p class="empty">
          Active app & game hours show up automatically as you work, draw or play.
        </p>
      </div>
    {:else}
      <ul class="mini-apps">
        {#each topApps as item (item.id)}
          {@const c = categoryInfo(item.category)}
          <li>
            <AppBadge name={item.name} category={item.category} iconUrl={item.icon_url} size={36} />
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
        <CatNap width={96} />
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
    <button class="tool-tile" style:--tile-glow={t.tint} onclick={() => app.go(t.id)}>
      <div class="tile-icon">
        <Icon name={t.icon} size={20} />
      </div>
      <div class="tile-title">{t.title}</div>
      <p class="tile-desc">{t.desc}</p>
    </button>
  {/each}
</section>

<style>
  .bento-main {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(330px, 1fr));
    gap: 14px;
    margin-bottom: 22px;
  }
  .bento-card {
    background: var(--surface-1);
    border: 1px solid var(--border);
    border-radius: var(--radius-l);
    padding: 18px 20px;
    box-shadow: var(--shadow-card);
    display: grid;
    align-content: start;
    gap: 14px;
  }
  .play-card {
    background:
      radial-gradient(240px 140px at 100% 0%, var(--glow-blush), transparent 72%),
      var(--surface-1);
  }
  .head {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
  }
  .sub {
    display: block;
    font-size: 12.5px;
    color: var(--text-3);
    margin-top: 2px;
  }
  .pill-link {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 5px 12px;
    border-radius: var(--radius-pill);
    border: 1px solid var(--border-strong);
    background: var(--surface-2);
    color: var(--accent-text);
    font-size: 12px;
    font-weight: 700;
    cursor: pointer;
    white-space: nowrap;
    transition: background var(--dur-1), border-color var(--dur-1);
  }
  .pill-link:hover {
    background: var(--surface-3);
    border-color: var(--border-glow);
  }

  .nap-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 6px 0;
  }
  .empty {
    margin: 0;
    color: var(--text-3);
    font-size: 13px;
  }

  .mini-apps, .dl-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 10px;
  }
  .mini-apps li {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 8px 10px;
    border-radius: var(--radius-m);
    background: var(--surface-2);
    border: 1px solid var(--border);
  }
  .m-body { flex: 1; min-width: 0; display: grid; gap: 5px; }
  .m-line { display: flex; justify-content: space-between; gap: 8px; font-size: 13px; }
  .m-name { font-weight: 600; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .m-track { height: 5px; border-radius: var(--radius-pill); background: var(--surface-1); overflow: hidden; }
  .m-fill { height: 100%; border-radius: var(--radius-pill); }

  .line { display: flex; justify-content: space-between; gap: 12px; margin-bottom: 6px; }
  .title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-weight: 600; }
  .state { color: var(--text-2); flex: none; font-size: 12.5px; }

  .grid-title {
    margin: 0 0 12px;
    font-size: 17px;
  }
  .tool-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(210px, 1fr));
    gap: 12px;
  }
  .tool-tile {
    display: grid;
    align-content: start;
    gap: 8px;
    padding: 16px 18px;
    border-radius: var(--radius-l);
    border: 1px solid var(--border);
    background:
      radial-gradient(160px 110px at 100% 0%, var(--tile-glow), transparent 75%),
      var(--surface-1);
    box-shadow: var(--shadow-card);
    text-align: left;
    cursor: pointer;
    transition: border-color var(--dur-2) var(--ease-out), background var(--dur-2) var(--ease-out);
  }
  .tool-tile:hover {
    border-color: var(--border-glow);
  }
  .tile-icon {
    width: 38px;
    height: 38px;
    border-radius: 12px;
    display: grid;
    place-items: center;
    background: var(--surface-2);
    border: 1px solid var(--border-strong);
    color: var(--accent);
  }
  .tile-title {
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 15.5px;
  }
  .tile-desc {
    margin: 0;
    font-size: 12.5px;
    color: var(--text-2);
    line-height: 1.45;
  }
</style>
