<script lang="ts">
  import Icon from '../../lib/icons/Icon.svelte';
  import { startupIpc, type StartupOverview } from '../../lib/ipc';
  import Switch from '../../lib/ui/Switch.svelte';
  import { delayText, impactOf, sortItems, type Impact } from './logic';

  let data = $state<StartupOverview | null>(null);
  let error = $state('');
  let busy = $state<string | null>(null);
  let filter = $state<'all' | 'slow' | 'on'>('all');

  async function load() {
    try {
      data = await startupIpc.list();
      error = '';
    } catch (e) {
      error = String(e);
    }
  }
  void load();

  const items = $derived(
    sortItems(data?.items ?? []).filter((i) => (filter === 'slow' ? impactOf(i.effect) !== 'none' : filter === 'on' ? i.enabled : true)),
  );
  const onCount = $derived((data?.items ?? []).filter((i) => i.enabled).length);

  async function toggle(id: string, enabled: boolean) {
    busy = id;
    error = '';
    try {
      await startupIpc.set(id, enabled);
      await load();
    } catch (e) {
      error = String(e);
    } finally {
      busy = null;
    }
  }

  const impactLabel: Record<Impact, string> = { high: 'Slows start-up a lot', medium: 'Slows start-up a little', low: 'Barely slows start-up', none: '' };
</script>

<header class="head">
  <h1><Icon name="power" size={24} /> Startup apps</h1>
  <p class="sub">What opens when you turn on your computer. Turning one off does not uninstall it: it just stops opening by itself, and you can turn it back on here.</p>
</header>

{#if data}
  <section class="summary card">
    <div class="stat"><strong class="tabular">{onCount}</strong><span>start with your computer</span></div>
    {#if data.last_boot_ms}
      <div class="stat"><strong class="tabular">{delayText(data.last_boot_ms)}</strong><span>the last start-up took</span></div>
    {/if}
    <div class="seg" role="tablist" aria-label="Show">
      <button role="tab" aria-selected={filter === 'all'} class:on={filter === 'all'} onclick={() => (filter = 'all')}>All</button>
      <button role="tab" aria-selected={filter === 'on'} class:on={filter === 'on'} onclick={() => (filter = 'on')}>Turned on</button>
      {#if data.has_record}<button role="tab" aria-selected={filter === 'slow'} class:on={filter === 'slow'} onclick={() => (filter = 'slow')}>Slowed it down</button>{/if}
    </div>
  </section>

  {#if !data.has_record}
    <p class="hint">Windows has not recorded which of these slowed the start-up down, so there are no times to show. Yours will appear after a few restarts.</p>
  {/if}

  <ul class="list">
    {#each items as i (i.id)}
      {@const impact = impactOf(i.effect)}
      <li class:off={!i.enabled}>
        <span class="ico">{#if i.icon_url}<img src={i.icon_url} alt="" />{:else}<Icon name="power" size={18} />{/if}</span>
        <div class="body">
          <div class="r1"><strong translate="no">{i.name}</strong><span class="from">{i.from}</span></div>
          <div class="cmd" translate="no" title={i.command}>{i.command}</div>
          {#if i.effect}
            <div class="effect {impact}">
              <Icon name="clock" size={13} />
              {impactLabel[impact]}: about {delayText(i.effect.total_ms / i.effect.times)} on average, {i.effect.times} {i.effect.times === 1 ? 'time' : 'times'} lately (worst {delayText(i.effect.worst_ms)}).
            </div>
          {/if}
        </div>
        <div class="sw">
          {#if i.can_change}
            <Switch checked={i.enabled} label={`${i.name} starts with the computer`} onchange={(on) => toggle(i.id, on)} />
          {:else}
            <span class="lock" title="Starts for every user, so only an administrator can change it."><Icon name="shield" size={15} /></span>
          {/if}
        </div>
      </li>
    {/each}
  </ul>
  {#if items.length === 0}<p class="hint">Nothing to show here.</p>{/if}
{:else if !error}
  <p class="hint">Looking…</p>
{/if}
{#if error}<p class="error" role="alert">{error}</p>{/if}

<style>
  .head h1 { display: flex; align-items: center; gap: 10px; margin: 0 0 4px; }
  .sub { margin: 0 0 16px; color: var(--text-2); max-width: 680px; }
  .card { max-width: 860px; margin-bottom: 14px; padding: 14px 18px; border-radius: var(--radius-m); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border-strong); box-shadow: var(--shadow-card); }
  .summary { display: flex; align-items: center; gap: 28px; flex-wrap: wrap; }
  .stat { display: grid; }
  .stat strong { font-family: var(--font-display); font-size: 24px; }
  .stat span { color: var(--text-3); font-size: 12.5px; }
  .seg { margin-left: auto; display: inline-flex; padding: 3px; border-radius: 99px; background: var(--surface-2); border: 1px solid var(--border); }
  .seg button { border: 0; background: none; color: var(--text-2); padding: 5px 13px; border-radius: 99px; cursor: pointer; font-weight: 600; font-size: 12.5px; }
  .seg button.on { background: var(--surface-3); color: var(--text); }
  .hint { color: var(--text-3); font-size: 12.5px; max-width: 760px; }
  .list { list-style: none; margin: 0; padding: 0; display: grid; gap: 8px; max-width: 860px; }
  li { display: flex; align-items: center; gap: 12px; padding: 10px 12px; border-radius: var(--radius-s); background: color-mix(in srgb, var(--surface-1) 92%, transparent); border: 1px solid var(--border); }
  li.off { opacity: 0.6; }
  .ico { flex: none; width: 36px; height: 36px; border-radius: 11px; display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); overflow: hidden; }
  .ico img { width: 26px; height: 26px; object-fit: contain; }
  .body { flex: 1; min-width: 0; display: grid; gap: 3px; }
  .r1 { display: flex; gap: 10px; align-items: baseline; flex-wrap: wrap; }
  .from { color: var(--text-3); font-size: 12px; }
  .cmd { color: var(--text-3); font-size: 12px; font-family: var(--font-mono); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .effect { display: flex; align-items: center; gap: 6px; font-size: 12.5px; }
  .effect.high { color: var(--danger); }
  .effect.medium { color: #ffc15c; }
  .effect.low { color: var(--text-2); }
  .sw { flex: none; }
  .lock { color: var(--text-3); display: grid; }
  .error { color: var(--danger); max-width: 860px; }
</style>
