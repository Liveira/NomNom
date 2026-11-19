<script lang="ts">
  import { app } from '../../app/state.svelte';
  import { formatBytes } from '../../lib/format';
  import Icon from '../../lib/icons/Icon.svelte';
  import { monitorIpc, noteIpc, reminderIpc, type MonSnapshot, type Note, type Reminder } from '../../lib/ipc';
  import { every } from '../../lib/poll';
  import ProgressBar from '../../lib/ui/ProgressBar.svelte';
  import { dueText } from '../reminders/logic';
  import { systemDisk } from './glance';

  let snap = $state<MonSnapshot | null>(null);
  let next = $state<Reminder | null>(null);
  let note = $state<Note | null>(null);

  const has = (id: string) => app.sidebarNav.some((n) => n.id === id);

  async function loadSnap() {
    snap = await monitorIpc.snapshot().catch(() => snap);
  }
  async function loadSmall() {
    if (has('reminders')) next = (await reminderIpc.list().catch(() => [])).find((r) => !r.done) ?? null;
    if (has('notes')) note = (await noteIpc.list('').catch(() => []))[0] ?? null;
  }

  $effect(() => {
    void loadSnap();
    void loadSmall();
    const a = every(5000, () => void loadSnap());
    const b = every(30_000, () => void loadSmall());
    return () => {
      a();
      b();
    };
  });

  const disk = $derived(snap ? systemDisk(snap.disks) : null);
  const ram = $derived(snap && snap.mem_total > 0 ? snap.mem_used / snap.mem_total : 0);
  const now = $derived(Math.floor(Date.now() / 1000));
</script>

<section class="glance" aria-label="At a glance">
  <div class="tile">
    <div class="lbl"><Icon name="gauge" size={15} /> Memory</div>
    {#if snap}
      <strong class="tabular">{Math.round(ram * 100)}%</strong>
      <ProgressBar value={ram} label="Memory in use" />
      <span class="sub tabular">{formatBytes(snap.mem_used)} of {formatBytes(snap.mem_total)}</span>
    {:else}<span class="sub">Reading…</span>{/if}
  </div>

  <div class="tile">
    <div class="lbl"><Icon name="storage" size={15} /> Disk</div>
    {#if disk}
      <strong class="tabular">{formatBytes(disk.total - disk.used)} free</strong>
      <ProgressBar value={disk.total > 0 ? disk.used / disk.total : 0} label="Disk space used" />
      <span class="sub tabular" translate="no">{disk.mount} · {formatBytes(disk.used)} of {formatBytes(disk.total)}</span>
    {:else}<span class="sub">Reading…</span>{/if}
  </div>

  {#if has('reminders')}
    <button class="tile click" onclick={() => app.go('reminders')}>
      <div class="lbl"><Icon name="bell" size={15} /> Next reminder</div>
      {#if next}
        <strong class="one" translate="no">{next.text}</strong>
        <span class="sub">{dueText(next.due_at, now)}</span>
      {:else}<span class="sub">Nothing waiting. Say "in 20 minutes stretch".</span>{/if}
    </button>
  {/if}

  {#if has('notes')}
    <button class="tile click" onclick={() => app.go('notes')}>
      <div class="lbl"><Icon name="pencil" size={15} /> Latest note</div>
      {#if note}
        <strong class="one" translate="no">{note.text.split('\n')[0]}</strong>
        <span class="sub">{note.pinned ? 'Pinned' : 'Notes'}</span>
      {:else}<span class="sub">No notes yet.</span>{/if}
    </button>
  {/if}
</section>

<style>
  .glance { display: grid; grid-template-columns: repeat(auto-fit, minmax(190px, 1fr)); gap: 12px; margin-bottom: 22px; }
  .tile { display: grid; align-content: start; gap: 6px; padding: 14px 16px; border-radius: var(--radius-m); background: var(--surface-1); border: 1px solid var(--border); box-shadow: var(--shadow-soft); text-align: left; color: inherit; font: inherit; }
  .click { cursor: pointer; }
  .click:hover { border-color: var(--border-glow); }
  .click:focus-visible { outline: 2px solid var(--focus); outline-offset: 2px; }
  .lbl { display: flex; align-items: center; gap: 6px; color: var(--text-3); font-size: 12.5px; font-weight: 700; }
  strong { font-size: 19px; }
  strong.one { font-size: 15px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .sub { color: var(--text-3); font-size: 12.5px; }
</style>
