<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import { DEFAULT_PICKS, TOOLS, withTool } from '../../lib/tools';
  import Switch from '../../lib/ui/Switch.svelte';

  const picked = $derived(app.settings?.appearance.sidebar_tools ?? null);
  const shown = (id: string) => picked === null || picked.includes(id);
  const count = $derived(TOOLS.filter((t) => shown(t.id)).length);
</script>

<header class="head">
  <div>
    <h1>Tools</h1>
    <p class="sub">Pick the tools you want in the sidebar. The others stay here, one click away, and you can add them any time.</p>
  </div>
  <div class="actions">
    <button class="plain" onclick={() => app.setSidebarTools(TOOLS.map((t) => t.id))}>Show all</button>
    <button class="plain" onclick={() => app.setSidebarTools(DEFAULT_PICKS)}>Just the basics</button>
  </div>
</header>

<p class="count" role="status">{count} of {TOOLS.length} tools in the sidebar</p>

<section class="grid">
  {#each TOOLS as t (t.id)}
    <article class="tool" class:off={!shown(t.id)} style:--tile-glow={t.tint}>
      <div class="top">
        <div class="tile-icon"><Icon name={t.icon} size={20} /></div>
        <Switch
          checked={shown(t.id)}
          label={`Show ${t.title} in the sidebar`}
          onchange={(v) => app.setSidebarTools(withTool(picked, t.id, v))}
        />
      </div>
      <h3>{t.title}</h3>
      <p class="desc">{t.desc}</p>
      <div class="foot">
        <span class="state">{shown(t.id) ? 'In the sidebar' : 'Not in the sidebar'}</span>
        <button class="open" onclick={() => app.go(t.id)}>Open <Icon name="arrow" size={14} /></button>
      </div>
    </article>
  {/each}
</section>

<style>
  .head { display: flex; justify-content: space-between; align-items: flex-start; gap: 24px; margin-bottom: 8px; }
  h1 { margin: 0 0 4px; }
  .sub { margin: 0; color: var(--text-2); max-width: 560px; }
  .actions { display: flex; gap: 8px; flex: none; }
  .count { color: var(--text-3); font-size: 12.5px; margin: 6px 0 14px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(244px, 1fr)); gap: 10px; }
  .tool {
    display: flex; flex-direction: column; gap: 6px; padding: 14px 16px;
    background: var(--surface-1);
    border: 1px solid var(--border); border-radius: var(--radius-m); box-shadow: var(--shadow-card);
    transition: opacity var(--dur-1), border-color var(--dur-1), background var(--dur-1);
  }
  .tool.off { opacity: 0.68; border-style: dashed; background: transparent; }
  .tool:hover { border-color: var(--border-strong); background: var(--surface-2); }
  .top { display: flex; justify-content: space-between; align-items: center; }
  .tile-icon { display: grid; place-items: center; width: 34px; height: 34px; border-radius: var(--radius-s); background: var(--surface-2); border: 1px solid var(--border); color: var(--accent-text); }
  h3 { margin: 6px 0 0; font-size: 14.5px; }
  .desc { margin: 0; color: var(--text-3); font-size: 12.5px; flex: 1; line-height: 1.45; }
  .foot { display: flex; justify-content: space-between; align-items: center; margin-top: 8px; }
  .state { color: var(--text-3); font-size: 12px; }
  .open { display: inline-flex; align-items: center; gap: 5px; border: 0; background: none; color: var(--accent-text); font-weight: 600; font-size: 13px; cursor: pointer; padding: 3px 2px; }
  .open:hover { text-decoration: underline; }
  button.plain { padding: 7px 14px; border-radius: var(--radius-s); border: 1px solid var(--border-strong); background: var(--surface-2); color: inherit; cursor: pointer; font-weight: 600; }
  button.plain:hover { background: var(--surface-3); }
  @media (max-width: 860px) { .head { flex-direction: column; } }
</style>
