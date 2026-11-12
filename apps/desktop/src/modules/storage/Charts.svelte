<script lang="ts">
  import { formatBytes } from '../../lib/format';
  import { stIpc, type StGroup, type StNode } from '../../lib/ipc';
  import { CATEGORY_COLORS, CATEGORY_LABELS } from './treemap';
  import { pieSlices } from './charts';
  import { storage } from './store.svelte';

  let folders = $state<StNode[]>([]);
  let categories = $state<StGroup[]>([]);
  let extensions = $state<StGroup[]>([]);
  let scope = $state<'all' | 'here'>('all');
  let error = $state('');
  let seq = 0;

  async function load() {
    const id = storage.sessionId;
    if (id == null) return;
    const node = scope === 'here' ? storage.current : 0;
    const mine = ++seq;
    try {
      const [f, s] = await Promise.all([stIpc.topFolders(id, node, 12, 'exclusive', 1), stIpc.stats(id, node)]);
      if (mine !== seq) return;
      folders = f;
      categories = s.categories.filter((c) => c.size > 0);
      extensions = s.extensions.slice(0, 12);
      error = '';
    } catch (e) {
      error = String(e);
    }
  }
  $effect(() => {
    void [storage.sessionId, storage.version, scope, storage.current];
    void load();
  });

  const slices = $derived(pieSlices(categories.map((c) => ({ key: c.key, value: c.size }))));
  const maxFolder = $derived(Math.max(1, ...folders.map((f) => f.size)));
  const maxExt = $derived(Math.max(1, ...extensions.map((e) => e.size)));
  const totalCat = $derived(categories.reduce((s, c) => s + c.size, 0));

  async function pickCategory(key: string) {
    storage.rankingCategory = key;
    storage.tab = 'rankings';
  }
  async function pickExtension(ext: string) {
    storage.rankingExtension = ext;
    storage.tab = 'rankings';
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
    <span class="hint">Click a slice or bar to see the files behind it.</span>
  </div>
  {#if error}<p class="error" role="alert">{error}</p>{/if}
  <div class="grid">
    <section aria-label="Space by kind of file">
      <h3>By kind of file</h3>
      {#if slices.length === 0}<p class="hint">Nothing to show yet.</p>{:else}
        <div class="pie">
          <svg viewBox="-1.05 -1.05 2.1 2.1" role="img" aria-label="Pie chart of space by kind of file">
            {#each slices as s (s.key)}
              <path d={s.path} fill={CATEGORY_COLORS[s.key] ?? CATEGORY_COLORS.other} class="slice" onclick={() => pickCategory(s.key)} role="button" tabindex="0" aria-label={`${CATEGORY_LABELS[s.key] ?? s.key}: ${formatBytes(categories.find((c) => c.key === s.key)?.size ?? 0)}`} onkeydown={(e) => (e.key === 'Enter' || e.key === ' ') && pickCategory(s.key)} />
            {/each}
          </svg>
          <ul>
            {#each categories as c (c.key)}
              <li>
                <button class="legendbtn" onclick={() => pickCategory(c.key)}>
                  <i style:background={CATEGORY_COLORS[c.key]}></i>
                  <span class="nm">{CATEGORY_LABELS[c.key] ?? c.key}</span>
                  <span class="tabular">{formatBytes(c.size)}</span>
                  <span class="tabular pct">{((c.size / Math.max(1, totalCat)) * 100).toFixed(1)}%</span>
                </button>
              </li>
            {/each}
          </ul>
        </div>
      {/if}
    </section>

    <section aria-label="Biggest folders">
      <h3>Biggest folders <span class="hint">(files directly inside, so nothing is counted twice)</span></h3>
      {#each folders as f (f.id)}
        <button class="bar" onclick={() => { storage.tab = 'explorer'; void storage.go(f.id); }} title={`Open ${f.name}`}>
          <span class="label">{f.name}</span>
          <span class="track"><span class="fill" style:width="{(f.size / maxFolder) * 100}%"></span></span>
          <span class="tabular val">{formatBytes(f.size)}</span>
        </button>
      {:else}<p class="hint">Nothing to show yet.</p>{/each}
    </section>

    <section aria-label="Biggest file types">
      <h3>By file type</h3>
      {#each extensions as e (e.key)}
        <button class="bar" onclick={() => pickExtension(e.key)} title={`Show .${e.key || 'no extension'} files`}>
          <span class="label">{e.key ? `.${e.key}` : 'no extension'}</span>
          <span class="track"><span class="fill" style:width="{(e.size / maxExt) * 100}%"></span></span>
          <span class="tabular val">{formatBytes(e.size)} <span class="hint">{e.files.toLocaleString()}</span></span>
        </button>
      {:else}<p class="hint">Nothing to show yet.</p>{/each}
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
  .bar { display: grid; grid-template-columns: 110px 1fr auto; gap: 8px; align-items: center; width: 100%; border: 0; background: none; color: inherit; padding: 3px 4px; border-radius: 6px; cursor: pointer; text-align: left; }
  .bar:hover { background: var(--surface-2); }
  .label { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .track { height: 8px; border-radius: 4px; background: var(--surface-3); overflow: hidden; }
  .fill { display: block; height: 100%; background: var(--accent); border-radius: 4px; }
  .val { color: var(--text-2); font-size: 12.5px; text-align: right; }
</style>
