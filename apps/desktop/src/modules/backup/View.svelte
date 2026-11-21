<script lang="ts">
  import { every } from '../../lib/poll';
  import { onDestroy } from 'svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { formatBytes } from '../../lib/format';
  import { backupIpc, onBackupChanged, onBackupProgress, type BackupPlan, type BackupPlanView, type BackupVersion } from '../../lib/ipc';
  import CatNap from '../../lib/ui/CatNap.svelte';
  import { ago, inTime, leaf, newPlan } from './logic';
  import PlanDialog from './PlanDialog.svelte';
  import RestoreDialog from './RestoreDialog.svelte';

  let plans = $state<BackupPlanView[]>([]);
  let editing = $state<BackupPlan | null>(null);
  let versionsOf = $state<{ plan: BackupPlan; list: BackupVersion[] } | null>(null);
  let restoring = $state<{ plan: BackupPlan; version: BackupVersion } | null>(null);
  let progress = $state<{ plan_id: string; files: number; bytes: number } | null>(null);
  let confirmRemove = $state<string | null>(null);
  let error = $state('');

  async function refresh() {
    plans = await backupIpc.plans();
    if (!plans.some((p) => p.running)) progress = null;
  }
  void refresh();
  let offs: (() => void)[] = [];
  void onBackupChanged(() => void refresh()).then((u) => offs.push(u));
  void onBackupProgress((p) => (progress = p)).then((u) => offs.push(u));
  const poll = every(30000, () => void refresh());
  onDestroy(() => {
    offs.forEach((o) => o());
    poll();
  });

  async function run(id: string) {
    error = '';
    try {
      await backupIpc.run(id);
      await refresh();
    } catch (e) {
      error = String(e);
    }
  }
  async function showVersions(p: BackupPlan) {
    error = '';
    try {
      versionsOf = { plan: p, list: await backupIpc.versions(p.id) };
    } catch (e) {
      error = String(e);
    }
  }
  async function remove(id: string) {
    await backupIpc.remove(id);
    confirmRemove = null;
    await refresh();
  }
  const anyRunning = $derived(plans.some((p) => p.running));
</script>

<header class="studio-hero">
  <div>
    <h1>Backup</h1>
    <p>Copy your important folders somewhere safe, on a schedule. Each backup is a dated version, and files that didn't change take no extra space.</p>
    <button class="primary" onclick={() => (editing = newPlan())}><Icon name="plus" size={16} /> New backup</button>
  </div>
</header>

{#if error}<p class="error" role="alert">{error}</p>{/if}

{#if plans.length === 0}
  <section class="empty">
    <CatNap width={110} />
    <div>
      <p class="big">No backups yet</p>
      <p class="hint">Make one to keep copies of your documents, pictures or projects. Your files are only read, never changed.</p>
    </div>
  </section>
{/if}

<div class="plans">
  {#each plans as v, i (v.plan.id)}
    <article class="plan" class:running={v.running} style:--i={i}>
      <div class="top">
        <div class="ico"><Icon name="backup" size={20} /></div>
        <div class="head">
          <h2 translate="no">{v.plan.name}</h2>
          <p class="srcs" translate="no" title={v.plan.sources.join('\n')}>{v.plan.sources.map(leaf).join(' · ')}</p>
        </div>
        <span class="badge" class:off={!v.plan.enabled || v.plan.schedule === 'manual'}>
          {#if v.plan.schedule === 'manual'}By hand{:else if !v.plan.enabled}Paused{:else}{v.plan.schedule === 'hourly' ? 'Every hour' : v.plan.schedule === 'daily' ? 'Every day' : 'Every week'}{/if}
        </span>
      </div>

      <div class="facts">
        <div>
          <span class="k">Last backup</span>
          {#if v.last}
            <strong class:bad={!v.last.ok}>{v.last.ok ? ago(v.last.started_at) : `Failed ${ago(v.last.started_at)}`}</strong>
            <span class="sub tabular">{v.last.ok ? `${v.last.files.toLocaleString()} files · ${formatBytes(v.last.bytes)} · ${formatBytes(v.last.copied)} new` : v.last.message ?? ''}</span>
          {:else}
            <strong>Never</strong>
          {/if}
        </div>
        <div>
          <span class="k">Next</span>
          <strong>{v.next_at ? inTime(v.next_at) : '—'}</strong>
          <span class="sub">keeps {v.plan.keep} versions</span>
        </div>
        <div>
          <span class="k">Kept in</span>
          <strong class="dest" translate="no" title={v.plan.destination}>{leaf(v.plan.destination)}</strong>
        </div>
      </div>

      {#if v.running}
        <div class="bar" role="progressbar" aria-label="Backing up"><i></i></div>
        <p class="sub tabular">{progress && progress.plan_id === v.plan.id ? `${progress.files.toLocaleString()} files · ${formatBytes(progress.bytes)}` : 'Starting…'}</p>
      {/if}
      {#if v.problem}<p class="warn" role="status">{v.problem}</p>{/if}

      <div class="actions">
        {#if v.running}
          <button class="plain" onclick={() => backupIpc.cancel()}><Icon name="pause" size={15} /> Stop</button>
        {:else}
          <button class="primary" onclick={() => run(v.plan.id)} disabled={anyRunning || !!v.problem}><Icon name="play" size={15} /> Back up now</button>
        {/if}
        <button class="plain" onclick={() => showVersions(v.plan)}><Icon name="clock" size={15} /> Versions</button>
        <button class="plain" onclick={() => (editing = v.plan)} disabled={v.running}><Icon name="settings" size={15} /> Edit</button>
        {#if confirmRemove === v.plan.id}
          <span class="sure">Forget this backup? The copies already made stay where they are.</span>
          <button class="plain" onclick={() => remove(v.plan.id)}>Yes, forget it</button>
          <button class="plain" onclick={() => (confirmRemove = null)}>Cancel</button>
        {:else}
          <button class="plain" onclick={() => (confirmRemove = v.plan.id)} disabled={v.running} aria-label="Forget this backup"><Icon name="trash" size={15} /></button>
        {/if}
      </div>
    </article>
  {/each}
</div>

{#if editing}
  <PlanDialog plan={editing} onclose={() => (editing = null)} onsaved={() => { editing = null; void refresh(); }} />
{/if}

{#if versionsOf}
  <div class="vpanel" role="dialog" aria-label="Versions">
    <div class="vhead">
      <h2>Versions of <span translate="no">{versionsOf.plan.name}</span></h2>
      <button class="plain" onclick={() => (versionsOf = null)}>Close</button>
    </div>
    {#if versionsOf.list.length === 0}
      <p class="hint">No versions yet. Press Back up now.</p>
    {:else}
      <ul>
        {#each versionsOf.list as ver, i (ver.name)}
          <li>
            <span class="vdot" class:newest={i === 0}></span>
            <span class="vwhen"><strong>{new Date(ver.started_at * 1000).toLocaleString()}</strong>{#if i === 0}<em>Newest</em>{/if}</span>
            <span class="vmeta tabular">{ver.files.toLocaleString()} files · {formatBytes(ver.bytes)}</span>
            <button class="plain" onclick={() => { const p = versionsOf!.plan; versionsOf = null; restoring = { plan: p, version: ver }; }}>Restore…</button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
{/if}

{#if restoring}
  <RestoreDialog plan={restoring.plan} version={restoring.version} onclose={() => (restoring = null)} />
{/if}

<style>
  .studio-hero p { max-width: 62ch; }
  .studio-hero button { margin-top: 12px; }
  .empty { display: flex; gap: 16px; align-items: center; padding: 18px 20px; border-radius: var(--radius-l); background: var(--surface-1); border: 1px dashed var(--border-strong); }
  .big { font-family: var(--font-display); font-weight: 700; font-size: 17px; margin: 0; }
  .hint { color: var(--text-2); margin: 4px 0 0; }
  .plans { display: grid; gap: 12px; }
  .plan { padding: 16px 18px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border); box-shadow: var(--shadow-card); display: grid; gap: 12px; }
  .plan.running { border-color: var(--accent); }
  .top { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 12px; align-items: center; }
  .ico { display: grid; place-items: center; width: 38px; height: 38px; border-radius: var(--radius-s); color: var(--accent-text); background: var(--surface-2); border: 1px solid var(--border); }
  h2 { font-size: 16px; }
  .srcs { margin: 2px 0 0; color: var(--text-3); font-size: 12.5px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .badge { padding: 3px 10px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); color: var(--text-2); font-size: 12px; font-weight: 600; }
  .badge.off { color: var(--text-3); }
  .facts { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 10px; }
  .facts > div { display: grid; gap: 1px; padding: 9px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); min-width: 0; }
  .k { font-size: 11px; text-transform: uppercase; letter-spacing: 0.06em; color: var(--text-3); font-weight: 600; }
  .sub { color: var(--text-3); font-size: 12px; }
  .bad { color: var(--danger); }
  .dest { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .bar { height: 4px; border-radius: 3px; background: var(--surface-3); overflow: hidden; }
  .bar i { display: block; height: 100%; width: 40%; border-radius: 3px; background: var(--accent); animation: slide 1.4s ease-in-out infinite; }
  @keyframes slide { 0% { transform: translateX(-100%); } 100% { transform: translateX(260%); } }
  .warn { color: var(--danger); margin: 0; font-size: 13px; }
  .actions { display: flex; flex-wrap: wrap; gap: 8px; align-items: center; }
  .sure { color: var(--text-2); font-size: 13px; }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; font-size: 13px; }
  button.plain:hover:not(:disabled) { background: var(--surface-3); }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
  button:disabled { opacity: 0.5; cursor: default; }
  .error { color: var(--danger); }

  .vpanel { position: fixed; inset: 0; z-index: 40; display: grid; place-content: center; background: rgba(0, 0, 0, 0.55); padding: 20px; }
  .vpanel > * { width: min(620px, 90vw); background: var(--surface-1); border: 1px solid var(--border-strong); padding: 18px 20px; }
  .vhead { display: flex; justify-content: space-between; align-items: center; gap: 12px; border-radius: var(--radius-l) var(--radius-l) 0 0; }
  .vpanel ul, .vpanel > p { border-radius: 0 0 var(--radius-l) var(--radius-l); margin: 0; border-top: 0; max-height: 52vh; overflow: auto; }
  .vpanel ul { list-style: none; display: grid; gap: 8px; }
  .vpanel li { display: grid; grid-template-columns: auto minmax(0, 1fr) auto auto; gap: 12px; align-items: center; }
  .vdot { width: 10px; height: 10px; border-radius: 50%; background: var(--text-3); }
  .vdot.newest { background: var(--ok); box-shadow: 0 0 8px var(--ok); }
  .vwhen { display: grid; }
  .vwhen em { color: var(--ok); font-style: normal; font-size: 12px; }
  .vmeta { color: var(--text-3); font-size: 12.5px; }
</style>
