<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import Icon from '../../lib/icons/Icon.svelte';
  import { backupIpc, type BackupPlan } from '../../lib/ipc';
  import Dialog from '../../lib/ui/Dialog.svelte';
  import { SCHEDULES, leaf } from './logic';

  let { plan, onclose, onsaved }: { plan: BackupPlan; onclose: () => void; onsaved: () => void } = $props();

  // svelte-ignore state_referenced_locally
  let draft = $state<BackupPlan>({ ...plan, sources: [...plan.sources] });
  let error = $state('');
  let busy = $state(false);

  async function addFolder() {
    const picked = await open({ directory: true, multiple: true });
    const list = Array.isArray(picked) ? picked : typeof picked === 'string' ? [picked] : [];
    for (const p of list) if (!draft.sources.includes(p)) draft.sources = [...draft.sources, p];
    if (!draft.name.trim() && draft.sources.length === 1) draft.name = leaf(draft.sources[0]);
  }
  async function pickDestination() {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === 'string') draft.destination = picked;
  }
  async function save() {
    busy = true;
    error = '';
    try {
      await backupIpc.save({ ...draft, keep: Math.round(Number(draft.keep)) });
      onsaved();
    } catch (e) {
      error = String(e);
    } finally {
      busy = false;
    }
  }
</script>

<!-- svelte-ignore state_referenced_locally -->
<Dialog title={plan.id ? 'Edit backup' : 'New backup'} {onclose}>
  <div class="form">
    <label class="f">
      <span class="l">Name</span>
      <input bind:value={draft.name} maxlength="60" placeholder="For example Photos" aria-label="Name" />
    </label>

    <div class="f">
      <span class="l">Folders to back up</span>
      {#if draft.sources.length > 0}
        <ul class="srcs">
          {#each draft.sources as s (s)}
            <li>
              <Icon name="folder" size={15} />
              <span translate="no" title={s}>{s}</span>
              <button class="x" aria-label="Remove this folder" onclick={() => (draft.sources = draft.sources.filter((x) => x !== s))}><Icon name="x" size={13} /></button>
            </li>
          {/each}
        </ul>
      {/if}
      <button class="plain" onclick={addFolder}><Icon name="plus" size={15} /> Add folders</button>
    </div>

    <div class="f">
      <span class="l">Keep the backups in</span>
      <div class="pick">
        <span class="path" translate="no" title={draft.destination}>{draft.destination || 'No folder chosen'}</span>
        <button class="plain" onclick={pickDestination}>Choose folder</button>
      </div>
      <span class="hint">An external drive is best. NomNom makes a folder named like this backup inside it.</span>
    </div>

    <div class="two">
      <label class="f">
        <span class="l">When</span>
        <select bind:value={draft.schedule} aria-label="When">
          {#each SCHEDULES as s (s.id)}<option value={s.id}>{s.label}</option>{/each}
        </select>
      </label>
      <label class="f">
        <span class="l">Versions to keep</span>
        <input type="number" min="1" max="100" bind:value={draft.keep} aria-label="Versions to keep" />
      </label>
    </div>

    <label class="f">
      <span class="l">Leave out</span>
      <input bind:value={draft.exclude} maxlength="300" placeholder="node_modules, .git, *.tmp" aria-label="Names to leave out" />
      <span class="hint">File or folder names, separated by commas. * stands for anything.</span>
    </label>

    {#if error}<p class="error" role="alert">{error}</p>{/if}
  </div>
  {#snippet actions()}
    <button class="plain" onclick={onclose}>Cancel</button>
    <button class="primary" onclick={save} disabled={busy}>{busy ? 'Saving…' : 'Save backup'}</button>
  {/snippet}
</Dialog>

<style>
  .form { display: grid; gap: 14px; min-width: min(560px, 80vw); }
  .f { display: grid; gap: 6px; }
  .l { font-weight: 700; font-size: 13px; }
  .hint { color: var(--text-3); font-size: 12.5px; }
  input, select { padding: 8px 12px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); user-select: text; }
  .two { display: grid; grid-template-columns: 1fr 160px; gap: 14px; }
  .srcs { list-style: none; margin: 0; padding: 0; display: grid; gap: 6px; }
  .srcs li { display: grid; grid-template-columns: auto minmax(0, 1fr) auto; gap: 8px; align-items: center; padding: 6px 10px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); }
  .srcs span { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .x { display: grid; place-items: center; width: 24px; height: 24px; border: 0; border-radius: 6px; background: var(--surface-3); color: inherit; cursor: pointer; }
  .pick { display: flex; gap: 10px; align-items: center; }
  .path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 8px 12px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px dashed var(--border-strong); color: var(--text-2); }
  button.plain, button.primary { display: inline-flex; align-items: center; gap: 6px; padding: 7px 16px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; width: fit-content; }
  button.primary { background: linear-gradient(135deg, var(--accent), var(--blush)); color: var(--on-accent); border-color: transparent; font-weight: 700; }
  button:disabled { opacity: 0.55; cursor: default; }
  .error { color: var(--danger); margin: 0; }
</style>
