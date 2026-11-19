<script lang="ts">
  import { listen } from '@tauri-apps/api/event';
  import { onMount, tick } from 'svelte';
  import Icon from '../lib/icons/Icon.svelte';
  import { toastIpc, type ToastCard } from '../lib/ipc';

  interface Shown extends ToastCard {
    id: number;
    ttl: number;
    leaving: boolean;
  }

  let cards = $state<Shown[]>([]);
  let stack: HTMLDivElement | undefined = $state();
  let seq = 0;
  let paused = $state(false);

  const ICON: Record<string, string> = { success: 'check', error: 'error', warning: 'warning', info: 'bell' };

  async function take() {
    const fresh = await toastIpc.take();
    for (const t of fresh) {
      cards.push({ ...t, id: ++seq, ttl: t.kind === 'error' ? 10000 : 6500, leaving: false });
    }
    await relayout();
  }

  async function relayout() {
    await tick();
    const h = stack?.offsetHeight ?? 0;
    if (h > 0) await toastIpc.layout(h);
  }

  function dismiss(id: number) {
    const card = cards.find((c) => c.id === id);
    if (!card || card.leaving) return;
    card.leaving = true;
    setTimeout(async () => {
      cards = cards.filter((c) => c.id !== id);
      if (cards.length === 0) {
        // Anything that arrived meanwhile is shown; otherwise the little window goes away.
        await take();
        if (cards.length === 0) await toastIpc.close();
      } else {
        await relayout();
      }
    }, 240);
  }

  function open(card: Shown) {
    if (card.action) void toastIpc.open(card.action);
    dismiss(card.id);
  }

  onMount(() => {
    // A window that was made for nothing (its cards went elsewhere) goes away again.
    void take().then(() => {
      if (cards.length === 0) setTimeout(() => cards.length === 0 && void toastIpc.close(), 1500);
    });
    const woke = listen('toast://wake', () => void take());
    // Each card leaves by itself, unless the pointer is on the stack.
    const timer = setInterval(() => {
      if (paused) return;
      for (const c of cards) {
        if (c.leaving) continue;
        c.ttl -= 250;
        if (c.ttl <= 0) dismiss(c.id);
      }
    }, 250);
    return () => {
      clearInterval(timer);
      void woke.then((off) => off());
    };
  });
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div class="stack" bind:this={stack} role="region" aria-label="Notifications" onpointerenter={() => (paused = true)} onpointerleave={() => (paused = false)}>
  {#each cards as c (c.id)}
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_static_element_interactions -->
    <div class="card {c.kind}" class:leaving={c.leaving} class:clickable={!!c.action} role="status" onclick={() => open(c)}>
      <span class="ico"><Icon name={ICON[c.kind] ?? 'bell'} size={19} stroke={2.2} /></span>
      <div class="text">
        <strong>{c.title}</strong>
        {#if c.body}<span>{c.body}</span>{/if}
      </div>
      <button class="x" aria-label="Dismiss" onclick={(e) => { e.stopPropagation(); dismiss(c.id); }}><Icon name="x" size={14} /></button>
      <i class="time" style="animation-duration:{c.kind === 'error' ? 10000 : 6500}ms" class:frozen={paused}></i>
    </div>
  {/each}
</div>

<style>
  :global(html), :global(body) { background: transparent; overflow: hidden; }
  .stack { display: flex; flex-direction: column; gap: 10px; padding: 6px; }
  .card {
    --tone: var(--accent);
    position: relative; display: flex; align-items: flex-start; gap: 12px; overflow: hidden;
    padding: 13px 14px 15px 14px; border-radius: 16px;
    background: linear-gradient(160deg, var(--surface-2), var(--surface-1));
    border: 1px solid var(--border-strong); box-shadow: var(--shadow-pop);
    animation: in 320ms var(--ease-spring);
  }
  .card.success { --tone: var(--ok); }
  .card.error { --tone: var(--danger); }
  .card.warning { --tone: #ffc15c; }
  .card.info { --tone: var(--info); }
  .card.clickable { cursor: pointer; }
  .card.clickable:hover { border-color: var(--tone); }
  .card.leaving { animation: out 240ms var(--ease-out) forwards; }
  .ico {
    flex: none; width: 36px; height: 36px; border-radius: 12px; display: grid; place-items: center;
    color: var(--tone); background: color-mix(in srgb, var(--tone) 16%, transparent);
  }
  .text { display: grid; gap: 2px; min-width: 0; flex: 1; padding-top: 1px; }
  strong { font-family: var(--font-display); font-size: 14px; color: var(--text); }
  .text span { color: var(--text-2); font-size: 12.5px; line-height: 1.35; display: -webkit-box; -webkit-line-clamp: 2; line-clamp: 2; -webkit-box-orient: vertical; overflow: hidden; }
  .x { flex: none; border: 0; background: none; color: var(--text-3); cursor: pointer; padding: 3px; border-radius: 8px; display: grid; }
  .x:hover { background: var(--surface-3); color: var(--text); }
  .time { position: absolute; left: 0; bottom: 0; height: 3px; width: 100%; background: var(--tone); opacity: 0.8; transform-origin: left; animation: drain linear forwards; }
  .time.frozen { animation-play-state: paused; }
  @keyframes in { from { opacity: 0; transform: translateX(26px) scale(0.96); } to { opacity: 1; transform: none; } }
  @keyframes out { to { opacity: 0; transform: translateX(26px) scale(0.96); } }
  @keyframes drain { from { transform: scaleX(1); } to { transform: scaleX(0); } }
  :global(html[data-opaque]) .card { animation-duration: 1ms; }
</style>
