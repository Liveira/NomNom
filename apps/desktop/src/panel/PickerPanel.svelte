<script lang="ts">
  import { onMount } from 'svelte';
  import { pickerIpc } from '../lib/ipc';

  const SPAN = 11; // pixels across the magnifier
  const ZOOM = 12;
  const SIZE = SPAN * ZOOM;

  let src = $state<string | null>(null);
  let img: HTMLImageElement | undefined = $state();
  let loupe: HTMLCanvasElement | undefined = $state();
  let at = $state<{ x: number; y: number } | null>(null);
  let hex = $state('#000000');
  /** The whole picture, to read single pixels from. */
  let source: CanvasRenderingContext2D | null = null;

  onMount(() => {
    void pickerIpc.image().then((s) => {
      if (s) src = s;
      else void pickerIpc.cancel();
    });
  });

  function loaded() {
    if (!img) return;
    const c = document.createElement('canvas');
    c.width = img.naturalWidth;
    c.height = img.naturalHeight;
    source = c.getContext('2d', { willReadFrequently: true });
    source?.drawImage(img, 0, 0);
    void pickerIpc.ready();
  }

  /** From the pointer in the window to a pixel of the picture (they differ on a scaled screen). */
  function pixel(e: PointerEvent): { x: number; y: number } {
    const k = (img?.naturalWidth ?? window.innerWidth) / window.innerWidth;
    return { x: Math.floor(e.clientX * k), y: Math.floor(e.clientY * k) };
  }

  function move(e: PointerEvent) {
    if (!source) return;
    at = { x: e.clientX, y: e.clientY };
    const p = pixel(e);
    const d = source.getImageData(p.x, p.y, 1, 1).data;
    hex = '#' + [d[0], d[1], d[2]].map((v) => v.toString(16).padStart(2, '0')).join('');
    const g = loupe?.getContext('2d');
    if (g) {
      g.imageSmoothingEnabled = false;
      g.clearRect(0, 0, SIZE, SIZE);
      const half = Math.floor(SPAN / 2);
      g.drawImage(source.canvas, p.x - half, p.y - half, SPAN, SPAN, 0, 0, SIZE, SIZE);
      g.strokeStyle = '#fff';
      g.lineWidth = 2;
      g.strokeRect(half * ZOOM, half * ZOOM, ZOOM, ZOOM);
      g.strokeStyle = '#000';
      g.lineWidth = 1;
      g.strokeRect(half * ZOOM - 1, half * ZOOM - 1, ZOOM + 2, ZOOM + 2);
    }
  }
  function click(e: PointerEvent) {
    move(e);
    void pickerIpc.done(hex);
  }
  function key(e: KeyboardEvent) {
    if (e.key === 'Escape') void pickerIpc.cancel();
  }

  // The magnifier sits beside the pointer and flips at the edges of the screen.
  const place = $derived.by(() => {
    if (!at) return { left: -999, top: -999 };
    const w = SIZE + 4;
    const h = SIZE + 40;
    const left = at.x + 24 + w > window.innerWidth ? at.x - 24 - w : at.x + 24;
    const top = at.y + 24 + h > window.innerHeight ? at.y - 24 - h : at.y + 24;
    return { left, top };
  });
</script>

<svelte:window onkeydown={key} />

{#if src}
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="stage" onpointermove={move} onpointerdown={click}>
    <img bind:this={img} {src} alt="" draggable="false" onload={loaded} />
    {#if at}
      <div class="loupe" style="left:{place.left}px; top:{place.top}px">
        <canvas bind:this={loupe} width={SIZE} height={SIZE}></canvas>
        <div class="code"><span class="chip" style="background:{hex}"></span>{hex}</div>
      </div>
    {/if}
    <div class="hint">Click a colour to copy it. Esc to cancel.</div>
  </div>
{/if}

<style>
  :global(html), :global(body) { margin: 0; background: #000; overflow: hidden; }
  .stage { position: fixed; inset: 0; cursor: crosshair; user-select: none; }
  img { position: absolute; inset: 0; width: 100%; height: 100%; display: block; }
  .loupe { position: absolute; border-radius: 10px; overflow: hidden; background: #16101f; border: 2px solid #fff; box-shadow: 0 6px 24px rgba(0, 0, 0, 0.55); pointer-events: none; }
  canvas { display: block; width: 132px; height: 132px; image-rendering: pixelated; }
  .code { display: flex; align-items: center; gap: 8px; padding: 6px 10px; color: #faf3fc; font: 600 13px ui-monospace, Consolas, monospace; }
  .chip { width: 16px; height: 16px; border-radius: 4px; border: 1px solid rgba(255, 255, 255, 0.6); }
  .hint { position: absolute; left: 50%; top: 22px; transform: translateX(-50%); padding: 8px 16px; border-radius: 99px; background: rgba(20, 14, 28, 0.9); color: #faf3fc; font: 600 14px system-ui, sans-serif; pointer-events: none; }
</style>
