<script lang="ts">
  import { app } from '../../app/state.svelte';
  import Backdrop from '../../lib/fx/Backdrop.svelte';
  import Icon from '../../lib/icons/Icon.svelte';
  import type { Motion, Theme } from '../../lib/ipc';
  import { isLinux } from '../../lib/platform';
  import { MOTIONS, THEME_GROUPS, THEMES_IN_ORDER } from './themes';

  const theme = $derived(app.settings?.appearance.theme ?? 'system');
  const motion = $derived(app.settings?.appearance.motion ?? 'full');
  let hovered = $state<string | null>(null);

  const renderer = $derived(app.settings?.general.renderer ?? 'auto');
  let rendererSaved = $state(false);
  const RENDERERS = [
    { id: 'auto', icon: 'sparkle', name: 'Automatic', hint: 'The graphics card, unless this computer needs the safe way.' },
    { id: 'gpu', icon: 'sliders', name: 'Graphics card', hint: 'Smoothest. Use it if Automatic feels slow.' },
    { id: 'cpu', icon: 'monitor', name: 'Processor only', hint: 'Never wakes the graphics card. Good for quiet fans.' },
  ] as const;
  async function pickRenderer(id: 'auto' | 'gpu' | 'cpu') {
    await app.update({ renderer: id });
    rendererSaved = true;
  }

  function pick(t: Theme, e: MouseEvent | KeyboardEvent) {
    const el = e.currentTarget as HTMLElement;
    const r = el.getBoundingClientRect();
    void app.setTheme(t, e instanceof MouseEvent && e.clientX ? { x: e.clientX, y: e.clientY } : { x: r.left + r.width / 2, y: r.top + r.height / 2 });
  }

  /** Arrow keys move between the theme cards, like any radio group. */
  function arrows(e: KeyboardEvent, i: number) {
    const all = THEMES_IN_ORDER.length;
    let next = -1;
    if (e.key === 'ArrowRight' || e.key === 'ArrowDown') next = (i + 1) % all;
    else if (e.key === 'ArrowLeft' || e.key === 'ArrowUp') next = (i - 1 + all) % all;
    if (next < 0) return;
    e.preventDefault();
    const cards = (e.currentTarget as HTMLElement).closest('.gallery')?.querySelectorAll<HTMLElement>('[role=radio]');
    cards?.[next]?.focus();
  }
</script>

<section class="group" aria-labelledby="themes">
  <h3 id="themes">Theme</h3>
  <p class="lead">Pick how NomNom looks. Move the pointer over a card to see its background move.</p>

  <div class="gallery" role="radiogroup" aria-label="Theme">
    {#each THEME_GROUPS as g (g.label)}
      <h4 class="gtitle">{g.label}</h4>
      <div class="grid">
      {#each g.items as t (t.id)}
        {@const i = THEMES_IN_ORDER.indexOf(t)}
        {@const on = theme === t.id}
        <button
          class="card-theme"
          class:on
          role="radio"
          aria-checked={on}
          tabindex={on || (theme === 'system' && i === 0) ? 0 : -1}
          style:--i={i}
          onclick={(e) => pick(t.id, e)}
          onkeydown={(e) => arrows(e, i)}
          onmouseenter={() => (hovered = t.id)}
          onmouseleave={() => (hovered = null)}
          onfocus={() => (hovered = t.id)}
          onblur={() => (hovered = null)}
        >
          <span class="mini" data-theme={t.id === 'system' ? 'dark' : t.id}>
            {#if t.id === 'system'}
              <span class="half" data-theme="light"></span>
            {/if}
            <Backdrop
              theme={t.id === 'system' ? 'dark' : t.id}
              motion={hovered === t.id || on ? (app.motion === 'off' ? 'off' : 'full') : 'calm'}
              contained
            />
            <span class="mock">
              <span class="mside"><i></i><i></i><i></i></span>
              <span class="mbody">
                <span class="mcard"><b></b><u></u></span>
                <span class="mbtn"></span>
              </span>
            </span>
          </span>
          <span class="meta">
            <span class="name"><Icon name={t.icon} size={15} /> {t.name}</span>
            <span class="tag">{t.tagline}</span>
          </span>
          {#if on}<span class="check" aria-hidden="true"><Icon name="check" size={13} stroke={3} /></span>{/if}
        </button>
      {/each}
      </div>
    {/each}
  </div>
</section>

<section class="group" aria-labelledby="motion">
  <h3 id="motion">Motion</h3>
  <p class="lead">How much NomNom moves. Animations pause by themselves when the window is hidden.</p>
  <div class="motions" role="radiogroup" aria-label="Motion">
    {#each MOTIONS as m (m.id)}
      {@const on = motion === m.id}
      <button class="mo" class:on role="radio" aria-checked={on} onclick={() => app.setMotion(m.id as Motion)}>
        <span class="mo-ico"><Icon name={m.icon} size={18} /></span>
        <span class="mo-text"><strong>{m.name}</strong><span>{m.hint}</span></span>
        {#if on}<span class="check" aria-hidden="true"><Icon name="check" size={13} stroke={3} /></span>{/if}
      </button>
    {/each}
  </div>
</section>

{#if isLinux}
  <section class="group" aria-labelledby="graphics">
    <h3 id="graphics">Graphics</h3>
    <p class="lead">How NomNom draws its window. If the fans spin up or it feels choppy, try another one.</p>
    <div class="motions" role="radiogroup" aria-label="Graphics">
      {#each RENDERERS as r (r.id)}
        {@const on = renderer === r.id}
        <button class="mo" class:on role="radio" aria-checked={on} onclick={() => void pickRenderer(r.id)}>
          <span class="mo-ico"><Icon name={r.icon} size={18} /></span>
          <span class="mo-text"><strong>{r.name}</strong><span>{r.hint}</span></span>
          {#if on}<span class="check" aria-hidden="true"><Icon name="check" size={13} stroke={3} /></span>{/if}
        </button>
      {/each}
    </div>
    {#if rendererSaved}<p class="lead" role="status">Saved. Close and open NomNom again to use it.</p>{/if}
  </section>
{/if}

<style>
  .lead { margin: 0 0 14px; color: var(--text-2); font-size: 13px; }
  .gallery { display: grid; gap: 10px; }
  .gtitle { margin: 10px 0 0; font-family: var(--font-ui); font-size: 11px; font-weight: 700; letter-spacing: 0.06em; text-transform: uppercase; color: var(--text-3); }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(186px, 1fr)); gap: 10px; }
  .card-theme {
    position: relative; display: grid; gap: 9px; padding: 9px; text-align: left; cursor: pointer;
    border-radius: var(--radius-m); border: 1px solid var(--border); background: var(--surface-2);
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .card-theme:hover { border-color: var(--border-strong); background: var(--surface-3); }
  .card-theme.on { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }

  .mini {
    position: relative; display: block; height: 92px; border-radius: var(--radius-xs); overflow: hidden;
    background: var(--bg); border: 1px solid var(--border); isolation: isolate;
  }
  .half { position: absolute; inset: 0; background: var(--bg); clip-path: polygon(100% 0, 100% 100%, 0 100%); z-index: 0; }
  .mock { position: absolute; inset: 7px; display: flex; gap: 5px; z-index: 2; }
  .mside { width: 18%; border-radius: 5px; background: var(--surface-1); border: 1px solid var(--border); padding: 5px 4px; display: grid; gap: 4px; align-content: start; }
  .mside i { height: 4px; border-radius: 2px; background: var(--surface-3); }
  .mside i:first-child { background: var(--accent); }
  .mbody { flex: 1; display: grid; gap: 5px; align-content: start; }
  .mcard { display: grid; gap: 4px; padding: 6px; border-radius: 6px; background: var(--surface-2); border: 1px solid var(--border); }
  .mcard b { height: 5px; width: 55%; border-radius: 2px; background: var(--text); }
  .mcard u { height: 4px; width: 80%; border-radius: 2px; background: var(--text-3); }
  .mbtn { height: 12px; width: 40%; border-radius: 5px; background: var(--accent); }

  .meta { display: grid; gap: 2px; padding: 0 2px 2px; }
  .name { display: inline-flex; align-items: center; gap: 6px; font-weight: 600; font-size: 13.5px; }
  .tag { font-size: 12px; color: var(--text-3); line-height: 1.35; }
  .check {
    position: absolute; top: 14px; right: 14px; width: 20px; height: 20px; border-radius: 50%; display: grid; place-items: center; z-index: 3;
    background: var(--accent); color: var(--on-accent);
  }

  .motions { display: grid; grid-template-columns: repeat(auto-fit, minmax(200px, 1fr)); gap: 10px; }
  .mo {
    position: relative; display: flex; align-items: center; gap: 11px; padding: 11px 13px; text-align: left; cursor: pointer;
    border-radius: var(--radius-m); border: 1px solid var(--border); background: var(--surface-2);
    transition: border-color var(--dur-1), background var(--dur-1);
  }
  .mo:hover { border-color: var(--border-strong); background: var(--surface-3); }
  .mo.on { border-color: var(--accent); box-shadow: 0 0 0 1px var(--accent); }
  .mo-ico { width: 32px; height: 32px; border-radius: var(--radius-s); display: grid; place-items: center; background: var(--surface-3); color: var(--accent-text); flex: none; }
  .mo.on .mo-ico { background: var(--accent); color: var(--on-accent); }
  .mo-text { display: grid; gap: 1px; }
  .mo-text span { font-size: 12px; color: var(--text-3); }
  .mo .check { top: 10px; right: 10px; }
</style>
