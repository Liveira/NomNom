<script lang="ts">
  import { open } from '@tauri-apps/plugin-dialog';
  import { formatBytes } from '../../lib/format';
  import { adminName, isLinux } from '../../lib/platform';
  import { driveKindLabel, driveTitle, usagePercent } from './logic';
  import QuickCleanup from './QuickCleanup.svelte';
  import { storage } from './store.svelte';

  let showOpts = $state(false);

  async function chooseFolder(elevated = false) {
    const picked = await open({ directory: true, multiple: false });
    if (typeof picked === 'string') await storage.start(picked, elevated);
  }
</script>

<section aria-label="Drives">
  <div class="head">
    <h2>Choose what to analyze</h2>
    <div class="buttons">
      <button class="plain" onclick={() => (showOpts = !showOpts)} aria-expanded={showOpts}>Scan options</button>
      <button class="plain" onclick={() => storage.openScanFile()}>Open saved scan…</button>
      <button class="plain" onclick={() => storage.loadDrives()} disabled={storage.drivesLoading}>Refresh drives</button>
      <button class="plain" onclick={() => chooseFolder(false)}>Choose a folder…</button>
      <button class="plain" onclick={() => chooseFolder(true)} title={isLinux ? 'Ask Polkit for root permission to read protected folders' : 'Ask Windows for Administrator permission to read protected folders'}>Folder as {isLinux ? 'root' : 'Admin'}…</button>
    </div>
  </div>

  {#if showOpts}
    <div class="opts" role="region" aria-label="Scan options">
      <label class="chk"><input type="checkbox" bind:checked={storage.scanCollectOwners} /> Read file owners</label>
      <label class="chk"><input type="checkbox" bind:checked={storage.scanCollectStreams} /> Check {isLinux ? 'extended attributes (xattr)' : 'NTFS alternate data streams'}</label>
      <label class="excl">Skip names or patterns
        <input bind:value={storage.scanExcludeText} placeholder="node_modules, .git, *.tmp" aria-label="Names or patterns to skip" />
      </label>
    </div>
  {/if}

  {#if storage.error}<p class="error" role="alert">{storage.error}</p>{/if}

  {#if storage.drives.length === 0}
    <p class="hint">{storage.drivesLoading ? 'Looking for drives…' : 'No drives found.'}</p>
  {/if}
  <div class="cards">
    {#each storage.drives as d (d.root)}
      {@const pct = usagePercent(d.used, d.total)}
      <div class="card" class:full={pct > 90}>
        <div class="top">
          <strong translate="no">{driveTitle(d)}</strong>
          <span class="kind">{driveKindLabel(d)}{d.filesystem ? ` · ${d.filesystem}` : ''}</span>
        </div>
        {#if d.total > 0}
          <div class="bar" role="img" aria-label={`${Math.round(pct)}% used`}><div class="fill" style:width="{pct}%"></div></div>
          <div class="nums tabular">
            <span>{formatBytes(d.used)} used</span>
            <span>{formatBytes(d.free)} free of {formatBytes(d.total)}</span>
          </div>
        {:else}
          <p class="hint">Not ready (no disk or no access).</p>
        {/if}
        <div class="cardbtns">
          <button class="primary grow" disabled={d.total === 0} onclick={() => storage.start(d.root, false)}>Analyze {d.root.startsWith('/') ? d.root : `${d.letter}:`}</button>
          <button class="plain" disabled={d.total === 0} onclick={() => storage.start(d.root, true)} title={`Scan as ${adminName} to include protected system folders`}>{isLinux ? 'As root' : 'As Admin'}</button>
        </div>
      </div>
    {/each}
  </div>
  <QuickCleanup />
</section>

<style>
  section { margin-bottom: 18px; }
  .head { display: flex; justify-content: space-between; align-items: center; margin-bottom: 10px; gap: 12px; flex-wrap: wrap; }
  h2 { font-size: 16px; }
  .buttons { display: flex; gap: 8px; flex-wrap: wrap; }
  .opts { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 10px 14px; margin-bottom: 12px; display: flex; flex-wrap: wrap; gap: 10px 20px; align-items: center; }
  .chk { display: inline-flex; gap: 6px; align-items: center; color: var(--text-2); font-size: 13px; cursor: pointer; }
  .excl { display: inline-flex; gap: 8px; align-items: center; color: var(--text-2); font-size: 13px; flex: 1; min-width: 260px; }
  .excl input { flex: 1; padding: 5px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .error { color: var(--danger); margin: 0 0 10px; }
  .cards { display: grid; grid-template-columns: repeat(auto-fill, minmax(260px, 1fr)); gap: 12px; }
  .card { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 14px 16px; display: grid; gap: 8px; }
  .top { display: flex; justify-content: space-between; gap: 8px; align-items: baseline; flex-wrap: wrap; }
  .kind { color: var(--text-3); font-size: 12.5px; }
  .bar { height: 8px; border-radius: 4px; background: var(--surface-3); overflow: hidden; }
  .fill { height: 100%; background: var(--accent); border-radius: 4px; }
  .full .fill { background: var(--danger); }
  .nums { display: flex; justify-content: space-between; gap: 8px; color: var(--text-2); font-size: 12.5px; }
  .cardbtns { display: flex; gap: 6px; }
  .grow { flex: 1; }
  .hint { color: var(--text-3); margin: 0; }
  button { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3); }
  button:disabled { opacity: 0.5; cursor: default; }
  button.primary { background: var(--accent); color: var(--on-accent); border-color: transparent; font-weight: 600; }
  button.primary:hover:not(:disabled) { background: var(--accent-hover); }
</style>
