<script lang="ts">
  import { onMount } from 'svelte';
  import { ocrIpc } from '../lib/ipc';

  let src = $state<string | null>(null);
  let img: HTMLImageElement | undefined = $state();
  let start = $state<{ x: number; y: number } | null>(null);
  let now = $state<{ x: number; y: number } | null>(null);
  let reading = $state(false);

  const box = $derived.by(() => {
    if (!start || !now) return null;
    return { x: Math.min(start.x, now.x), y: Math.min(start.y, now.y), w: Math.abs(now.x - start.x), h: Math.abs(now.y - start.y) };
  });

  onMount(() => {
    void ocrIpc.image().then((s) => {
      if (s) src = s;
      else void ocrIpc.cancel();
    });
  });

  async function loaded() {
    await ocrIpc.ready();
  }

  function down(e: PointerEvent) {
    if (reading) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    start = { x: e.clientX, y: e.clientY };
    now = start;
  }
  function move(e: PointerEvent) {
    if (start && !reading) now = { x: e.clientX, y: e.clientY };
  }
  async function up(e: PointerEvent) {
    if (!start || !box || !img) return;
    now = { x: e.clientX, y: e.clientY };
    const b = box;
    if (b.w < 6 || b.h < 6) {
      start = now = null;
      return;
    }
    // The picture is in screen pixels and the window in CSS pixels.
    const k = img.naturalWidth / window.innerWidth;
    reading = true;
    try {
      await ocrIpc.run(Math.round(b.x * k), Math.round(b.y * k), Math.round(b.w * k), Math.round(b.h * k));
    } catch {
      await ocrIpc.cancel();
    }
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') void ocrIpc.cancel();
  }
</script>

<svelte:window onkeydown={key} />

{#if src}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="stage" onpointerdown={down} onpointermove={move} onpointerup={up}>
    <img bind:this={img} {src} alt="" draggable="false" onload={loaded} />
    <div class="dim"></div>
    {#if box}
      <div class="sel" style="left:{box.x}px; top:{box.y}px; width:{box.w}px; height:{box.h}px"></div>
    {/if}
    <div class="hint" aria-live="polite">{reading ? 'Reading…' : 'Drag over the text. Esc to cancel.'}</div>
  </div>
{/if}

<style>
  :global(html), :global(body) { margin: 0; background: #000; overflow: hidden; }
  .stage { position: fixed; inset: 0; cursor: crosshair; user-select: none; }
  img { position: absolute; inset: 0; width: 100%; height: 100%; display: block; }
  .dim { position: absolute; inset: 0; background: rgba(0, 0, 0, 0.38); pointer-events: none; }
  .sel { position: absolute; border: 2px solid var(--accent, #ffa970); box-shadow: 0 0 0 100vmax rgba(0, 0, 0, 0.38); background: transparent; pointer-events: none; }
  .hint { position: absolute; left: 50%; top: 22px; transform: translateX(-50%); padding: 8px 16px; border-radius: 99px; background: rgba(20, 14, 28, 0.9); color: #faf3fc; font: 600 14px system-ui, sans-serif; pointer-events: none; }
</style>
