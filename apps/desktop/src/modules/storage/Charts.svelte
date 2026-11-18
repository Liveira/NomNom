<script lang="ts">
  import { stIpc, stTools, type StDateField, type StGroup, type StNode } from '../../lib/ipc';
  import { formatUnitBytes } from './logic';
  import { newRow } from './queryBuilder';
  import { CATEGORY_COLORS, CATEGORY_LABELS } from './treemap';
  import { pieSlices } from './charts';
  import { storage } from './store.svelte';

  let folders = $state<StNode[]>([]);
  let categories = $state<StGroup[]>([]);
  let extensions = $state<StGroup[]>([]);
  let owners = $state<StGroup[]>([]);
  let sizes = $state<StGroup[]>([]);
  let ages = $state<StGroup[]>([]);
  let ageField = $state<StDateField>('modified');
  let sizeBy = $state<'bytes' | 'count'>('bytes');
  let scope = $state<'all' | 'here'>('all');
  let error = $state('');
  let seq = 0;

  async function load() {
    const id = storage.sessionId;
    if (id == null) return;
    const node = scope === 'here' ? storage.current : 0;
    const mine = ++seq;
    try {
      const [f, s, a] = await Promise.all([
        stIpc.topFolders(id, node, 12, 'exclusive', 1),
        stIpc.stats(id, node),
        stTools.byAge(id, node, ageField),
      ]);
      if (mine !== seq) return;
      folders = f;
      ages = a;
      categories = s.categories.filter((c) => c.size > 0);
      extensions = s.extensions.slice(0, 12);
      owners = (s.owners ?? []).slice(0, 12);
      sizes = s.sizes ?? [];
      error = '';
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    void [storage.sessionId, storage.version, scope, storage.current, ageField];
    void load();
  });

  const slices = $derived(pieSlices(categories.map((c) => ({ key: c.key, value: c.size }))));
  const maxFolder = $derived(Math.max(1, ...folders.map((f) => f.size)));
  const maxExt = $derived(Math.max(1, ...extensions.map((e) => e.size)));
  const maxOwner = $derived(Math.max(1, ...owners.map((o) => o.size)));
  const maxAge = $derived(Math.max(1, ...ages.map((a) => a.size)));
  const maxBucket = $derived(Math.max(1, ...sizes.map((b) => (sizeBy === 'bytes' ? b.size : b.files))));
  const AGE_FIELDS: Record<StDateField, string> = { modified: 'last changed', created: 'created', accessed: 'last opened' };
  const totalCat = $derived(categories.reduce((s, c) => s + c.size, 0));

  function pickCategory(key: string) {
    storage.rankingCategory = key;
    storage.tab = 'rankings';
  }
  function pickExtension(ext: string) {
    storage.rankingExtension = ext;
    storage.tab = 'rankings';
  }
  function pickOwner(owner: string) {
    if (!owner || owner === '(unknown)') return;
    storage.jumpToSearch([{ ...newRow('item'), what: 'file' }, { ...newRow('owner'), text: owner, mode: 'contains' }]);
  }
  function pickAge(bucketKey: string) {
    const fileRow = { ...newRow('item'), what: 'file' as const };
    switch (bucketKey) {
      case 'Last 7 days':
        storage.jumpToSearch([fileRow, { ...newRow('age'), field: ageField, dir: 'newer', days: '7' }]);
        break;
      case '8 to 30 days':
        storage.jumpToSearch([fileRow, { ...newRow('age'), field: ageField, dir: 'older', days: '7' }, { ...newRow('age'), field: ageField, dir: 'newer', days: '30' }]);
        break;
      case '31 to 90 days':
        storage.jumpToSearch([fileRow, { ...newRow('age'), field: ageField, dir: 'older', days: '30' }, { ...newRow('age'), field: ageField, dir: 'newer', days: '90' }]);
        break;
      case '3 months to 1 year':
        storage.jumpToSearch([fileRow, { ...newRow('age'), field: ageField, dir: 'older', days: '90' }, { ...newRow('age'), field: ageField, dir: 'newer', days: '365' }]);
        break;
      case '1 to 3 years':
        storage.jumpToSearch([fileRow, { ...newRow('age'), field: ageField, dir: 'older', days: '365' }, { ...newRow('age'), field: ageField, dir: 'newer', days: '1095' }]);
        break;
      case 'Over 3 years':
        storage.jumpToSearch([fileRow, { ...newRow('age'), field: ageField, dir: 'older', days: '1095' }]);
        break;
    }
  }
  function pickSizeBucket(key: string) {
    const fileRow = { ...newRow('item'), what: 'file' as const };
    const map: Record<string, { min: string; max: string; unit: 'KB' | 'MB' | 'GB' }> = {
      'Empty (0 B)': { min: '0', max: '0', unit: 'KB' },
      'Tiny (< 4 KB)': { min: '', max: '4', unit: 'KB' },
      'Small (4 – 64 KB)': { min: '4', max: '64', unit: 'KB' },
      'Medium (64 KB – 1 MB)': { min: '64', max: '1024', unit: 'KB' },
      'Large (1 – 16 MB)': { min: '1', max: '16', unit: 'MB' },
      'Very large (16 – 100 MB)': { min: '16', max: '100', unit: 'MB' },
      'Huge (100 MB – 1 GB)': { min: '100', max: '1024', unit: 'MB' },
      'Over 1 GB': { min: '1', max: '', unit: 'GB' },
    };
    const spec = map[key];
    if (spec) storage.jumpToSearch([fileRow, { ...newRow('size'), ...spec }]);
  }
</script>

<div class="charts">
  <div class="top">
    <label>Where
      <select bind:value={scope}>
        <option value="all">Whole scan</option>
        <option value="here">Current folder only</option>
      </select>
    </label>
    <span class="hint">Click any slice or bar to inspect the matching files.</span>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="grid">
    <section aria-label="Space by kind of file">
      <h3>By kind of file</h3>
      {#if slices.length === 0}<p class="hint">Nothing to show yet.</p>{:else}
        <div class="pie">
          <svg viewBox="-1.05 -1.05 2.1 2.1" role="img" aria-label="Pie chart of space by kind of file">
            {#each slices as s (s.key)}
              <path d={s.path} fill={CATEGORY_COLORS[s.key] ?? CATEGORY_COLORS.other} class="slice" onclick={() => pickCategory(s.key)} role="button" tabindex="0" aria-label={`${CATEGORY_LABELS[s.key] ?? s.key}: ${formatUnitBytes(categories.find((c) => c.key === s.key)?.size ?? 0, storage.unit)}`} onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && pickCategory(s.key)} />
            {/each}
          </svg>
          <ul>
            {#each categories as c (c.key)}
              <li>
                <button class="legendbtn" onclick={() => pickCategory(c.key)}>
                  <i style:background={CATEGORY_COLORS[c.key]}></i>
                  <span class="nm">{CATEGORY_LABELS[c.key] ?? c.key}</span>
                  <span class="tabular">{formatUnitBytes(c.size, storage.unit)}</span>
                  <span class="tabular pct">{((c.size / Math.max(1, totalCat)) * 100).toFixed(1)}%</span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </section>

    <section aria-label="Biggest folders">
      <h3>Biggest folders <span class="hint">(files directly inside, non-overlapping)</span></h3>
      {#each folders as f (f.id)}
        <button class="bar" onclick={() => { storage.tab = 'explorer'; void storage.go(f.id); }} title={`Open ${f.name}`}>
          <span class="label">{f.name}</span>
          <span class="track"><span class="fill" style:width="{(f.size / maxFolder) * 100}%"></span></span>
          <span class="tabular val">{formatUnitBytes(f.size, storage.unit)}</span>
        </button>
      {:else}<p class="hint">Nothing to show yet.</p>{/each}
    </section>

    <section aria-label="Biggest file types">
      <h3>By file type</h3>
      {#each extensions as e (e.key)}
        <button class="bar" onclick={() => pickExtension(e.key)} title={`Show .${e.key || 'no extension'} files`}>
          <span class="label">{e.key ? `.${e.key}` : 'no extension'}</span>
          <span class="track"><span class="fill" style:width="{(e.size / maxExt) * 100}%"></span></span>
          <span class="tabular val">{formatUnitBytes(e.size, storage.unit)} <span class="hint">{e.files.toLocaleString()}</span></span>
        </button>
      {:else}<p class="hint">Nothing to show yet.</p>{/each}
    </section>

    <section aria-label="Files by age">
      <h3>By age
        <select bind:value={ageField} aria-label="Which date">
          {#each Object.entries(AGE_FIELDS) as [k, label] (k)}<option value={k}>{label}</option>{/each}
        </select>
      </h3>
      {#each ages.filter((a) => a.files > 0) as a (a.key)}
        <button class="bar" onclick={() => pickAge(a.key)} disabled={a.key === 'No date'} title={`Search files from ${a.key}`}>
          <span class="label">{a.key}</span>
          <span class="track"><span class="fill" style:width="{(a.size / maxAge) * 100}%"></span></span>
          <span class="tabular val">{formatUnitBytes(a.size, storage.unit)} <span class="hint">{a.files.toLocaleString()}</span></span>
        </button>
      {:else}<p class="hint">Nothing to show yet.</p>{/each}
      {#if ageField === 'accessed'}<p class="hint">Windows often doesn't update the last-opened date, so this one can look older than it is.</p>{/if}
    </section>

    <section aria-label="Files by size range">
      <h3>By file size
        <select bind:value={sizeBy} aria-label="Show space or file count">
          <option value="bytes">space used</option>
          <option value="count">number of files</option>
        </select>
      </h3>
      {#each sizes.filter((b) => b.files > 0) as b (b.key)}
        {@const val = sizeBy === 'bytes' ? b.size : b.files}
        <button class="bar" onclick={() => pickSizeBucket(b.key)} title={`Search files in ${b.key}`}>
          <span class="label">{b.key}</span>
          <span class="track"><span class="fill" style:width="{(val / maxBucket) * 100}%"></span></span>
          <span class="tabular val">{formatUnitBytes(b.size, storage.unit)} <span class="hint">{b.files.toLocaleString()}</span></span>
        </button>
      {:else}<p class="hint">Nothing to show yet.</p>{/each}
    </section>

    <section aria-label="Space by owner">
      <h3>By owner <span class="hint">(NTFS account)</span></h3>
      {#each owners.filter((o) => o.files > 0) as o (o.key)}
        <button class="bar" onclick={() => pickOwner(o.key)} disabled={o.key === '(unknown)'} title={`Search files owned by ${o.key}`}>
          <span class="label" title={o.key}>{o.key}</span>
          <span class="track"><span class="fill" style:width="{(o.size / maxOwner) * 100}%"></span></span>
          <span class="tabular val">{formatUnitBytes(o.size, storage.unit)} <span class="hint">{o.files.toLocaleString()}</span></span>
        </button>
      {:else}<p class="hint">No owner data collected for this scan.</p>{/each}
    </section>
  </div>
</div>

<style>
  .charts { height: 100%; overflow: auto; }
  .top { display: flex; gap: 16px; align-items: center; margin-bottom: 10px; }
  label { display: flex; gap: 6px; align-items: center; color: var(--text-2); font-size: 12.5px; }
  select { padding: 4px 8px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: var(--text); }
  .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(340px, 1fr)); gap: 14px; }
  section { background: var(--surface-1); border: 1px solid var(--border); border-radius: var(--radius-m); padding: 12px 14px; min-width: 0; }
  h3 { font-size: 14px; margin-bottom: 8px; }
  .hint { color: var(--text-3); font-size: 12px; font-weight: 400; }
  .error { color: var(--danger); }
  .pie { display: grid; grid-template-columns: 150px 1fr; gap: 14px; align-items: center; }
  svg { width: 150px; height: 150px; }
  .slice { cursor: pointer; stroke: var(--surface-1); stroke-width: 0.02; transition: opacity var(--dur-1); }
  .slice:hover, .slice:focus-visible { opacity: 0.8; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 2px; }
  .legendbtn { display: grid; grid-template-columns: 12px 1fr auto 48px; gap: 8px; align-items: center; width: 100%; border: 0; background: none; color: inherit; padding: 3px 4px; border-radius: 6px; cursor: pointer; text-align: left; }
  .legendbtn:hover { background: var(--surface-2); }
  .legendbtn i { width: 10px; height: 10px; border-radius: 2px; }
  .pct { color: var(--text-3); text-align: right; }
  .bar { display: grid; grid-template-columns: 126px 1fr auto; gap: 8px; align-items: center; width: 100%; border: 0; background: none; color: inherit; padding: 3px 4px; border-radius: 6px; cursor: pointer; text-align: left; }
  .bar:hover:not(:disabled) { background: var(--surface-2); }
  .bar:disabled { cursor: default; }
  h3 select { margin-left: 8px; font-size: 12.5px; }
  .label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { height: 8px; border-radius: 4px; background: var(--surface-3); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); border-radius: 4px; }
  .val { color: var(--text-2); font-size: 12.5px; text-align: right; }
</style>
