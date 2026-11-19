<script lang="ts">
  import { save } from '@tauri-apps/plugin-dialog';
  import { onMount, tick } from 'svelte';
  import { shotIpc } from '../lib/ipc';
  import {
    COLORS, WIDTHS, arrowParts, blockSize, boxOf, clampBox, isReal, nextStep, stepRadius, textSize,
    type Pt, type Shape,
  } from '../modules/shots/draw';

  type Tool = 'pen' | 'arrow' | 'rect' | 'mark' | 'blur' | 'step' | 'text';
  const TOOLS: { id: Tool; label: string; hint: string }[] = [
    { id: 'arrow', label: 'Arrow', hint: 'Drag from the tail to the point' },
    { id: 'rect', label: 'Box', hint: 'Drag around something' },
    { id: 'mark', label: 'Marker', hint: 'Drag over text to highlight it' },
    { id: 'blur', label: 'Hide', hint: 'Drag over what must not be seen: it is blocked out for good' },
    { id: 'pen', label: 'Pen', hint: 'Draw freely' },
    { id: 'step', label: 'Step', hint: 'Click to put numbers 1, 2, 3 in order' },
    { id: 'text', label: 'Text', hint: 'Click where the words go' },
  ];

  let canvas: HTMLCanvasElement | undefined = $state();
  let base: HTMLImageElement | null = null;
  let tool = $state<Tool>('arrow');
  let color = $state(COLORS[0]);
  let width = $state(WIDTHS[1]);
  let shapes = $state<Shape[]>([]);
  let undone = $state<Shape[]>([]);
  let draft = $state<Shape | null>(null);
  let dragFrom: Pt | null = null;
  let typing = $state<{ at: Pt; css: Pt; text: string } | null>(null);
  let typer: HTMLInputElement | undefined = $state();
  let note = $state('');
  let ready = $state(false);

  onMount(() => {
    void shotIpc.editImage().then((src) => {
      if (!src) return void shotIpc.close();
      const img = new Image();
      img.onload = async () => {
        base = img;
        ready = true;
        // The canvas exists only once the page has drawn it; its size is the picture's own.
        await tick();
        if (canvas) {
          canvas.width = img.naturalWidth;
          canvas.height = img.naturalHeight;
        }
        paint();
        void shotIpc.editorReady();
      };
      img.src = src;
    });
  });

  $effect(() => {
    void shapes;
    void draft;
    if (ready) paint();
  });

  function pixelate(g: CanvasRenderingContext2D, box: { x: number; y: number; w: number; h: number }) {
    if (!base || box.w < 1 || box.h < 1) return;
    const b = blockSize(box);
    const sw = Math.max(1, Math.round(box.w / b));
    const sh = Math.max(1, Math.round(box.h / b));
    const tiny = document.createElement('canvas');
    tiny.width = sw;
    tiny.height = sh;
    const t = tiny.getContext('2d');
    if (!t) return;
    t.drawImage(base, box.x, box.y, box.w, box.h, 0, 0, sw, sh);
    g.save();
    g.imageSmoothingEnabled = false;
    g.drawImage(tiny, 0, 0, sw, sh, box.x, box.y, box.w, box.h);
    g.restore();
  }

  function draw(g: CanvasRenderingContext2D, s: Shape) {
    g.save();
    g.lineCap = 'round';
    g.lineJoin = 'round';
    switch (s.kind) {
      case 'pen': {
        g.strokeStyle = s.color;
        g.lineWidth = s.width;
        g.beginPath();
        s.points.forEach((p, i) => (i === 0 ? g.moveTo(p.x, p.y) : g.lineTo(p.x, p.y)));
        if (s.points.length === 1) g.lineTo(s.points[0].x + 0.1, s.points[0].y);
        g.stroke();
        break;
      }
      case 'arrow': {
        const a = arrowParts(s.from, s.to, s.width);
        g.strokeStyle = s.color;
        g.fillStyle = s.color;
        g.lineWidth = s.width;
        if (a) {
          g.beginPath();
          g.moveTo(s.from.x, s.from.y);
          g.lineTo(a.base.x, a.base.y);
          g.stroke();
          g.beginPath();
          g.moveTo(a.tip.x, a.tip.y);
          g.lineTo(a.left.x, a.left.y);
          g.lineTo(a.right.x, a.right.y);
          g.closePath();
          g.fill();
        }
        break;
      }
      case 'rect':
        g.strokeStyle = s.color;
        g.lineWidth = s.width;
        g.strokeRect(s.box.x, s.box.y, s.box.w, s.box.h);
        break;
      case 'mark':
        g.fillStyle = s.color + '66';
        g.fillRect(s.box.x, s.box.y, s.box.w, s.box.h);
        break;
      case 'blur':
        pixelate(g, s.box);
        break;
      case 'step': {
        const r = stepRadius(s.width);
        g.fillStyle = s.color;
        g.beginPath();
        g.arc(s.at.x, s.at.y, r, 0, Math.PI * 2);
        g.fill();
        g.lineWidth = 2;
        g.strokeStyle = '#ffffff';
        g.stroke();
        g.fillStyle = s.color === '#ffffff' || s.color === '#ffb020' ? '#111111' : '#ffffff';
        g.font = `700 ${Math.round(r * 1.15)}px system-ui, sans-serif`;
        g.textAlign = 'center';
        g.textBaseline = 'middle';
        g.fillText(String(s.n), s.at.x, s.at.y + 1);
        break;
      }
      case 'text': {
        const size = textSize(s.width);
        g.font = `700 ${size}px system-ui, sans-serif`;
        g.textBaseline = 'top';
        g.lineWidth = Math.max(3, size / 6);
        g.strokeStyle = s.color === '#111111' ? '#ffffff' : '#000000';
        g.strokeText(s.text, s.at.x, s.at.y);
        g.fillStyle = s.color;
        g.fillText(s.text, s.at.x, s.at.y);
        break;
      }
    }
    g.restore();
  }

  function paint() {
    const g = canvas?.getContext('2d');
    if (!g || !base || !canvas) return;
    g.clearRect(0, 0, canvas.width, canvas.height);
    g.drawImage(base, 0, 0);
    for (const s of shapes) draw(g, s);
    if (draft) draw(g, draft);
  }

  /** The pointer in picture pixels (the canvas is shown smaller or bigger than it is). */
  function at(e: PointerEvent): Pt {
    const r = canvas!.getBoundingClientRect();
    return { x: ((e.clientX - r.left) * canvas!.width) / r.width, y: ((e.clientY - r.top) * canvas!.height) / r.height };
  }

  function down(e: PointerEvent) {
    if (!canvas || !ready) return;
    const p = at(e);
    if (tool === 'step') {
      push({ kind: 'step', color, width, at: p, n: nextStep(shapes) });
      return;
    }
    if (tool === 'text') {
      const r = canvas.getBoundingClientRect();
      typing = { at: p, css: { x: e.clientX - r.left, y: e.clientY - r.top }, text: '' };
      void tick().then(() => typer?.focus());
      return;
    }
    canvas.setPointerCapture(e.pointerId);
    dragFrom = p;
    draft = start(p, p);
  }
  function start(a: Pt, b: Pt): Shape | null {
    switch (tool) {
      case 'pen':
        return { kind: 'pen', color, width, points: [a] };
      case 'arrow':
        return { kind: 'arrow', color, width, from: a, to: b };
      case 'rect':
        return { kind: 'rect', color, width, box: boxOf(a, b) };
      case 'mark':
        return { kind: 'mark', color: color, box: boxOf(a, b) };
      case 'blur':
        return { kind: 'blur', box: boxOf(a, b) };
      default:
        return null;
    }
  }
  function move(e: PointerEvent) {
    if (!dragFrom || !draft) return;
    const p = at(e);
    draft = draft.kind === 'pen' ? { ...draft, points: [...draft.points, p] } : start(dragFrom, p);
  }
  function up(e: PointerEvent) {
    if (!dragFrom || !draft) return;
    const p = at(e);
    let done: Shape | null = draft.kind === 'pen' ? { ...draft, points: [...draft.points, p] } : start(dragFrom, p);
    dragFrom = null;
    draft = null;
    if (!done || !canvas) return;
    if (done.kind === 'rect' || done.kind === 'mark' || done.kind === 'blur') {
      const box = clampBox(done.box, canvas.width, canvas.height);
      if (!isReal(box)) return;
      done = { ...done, box };
    } else if (done.kind === 'arrow' && !arrowParts(done.from, done.to, done.width)) {
      return;
    }
    push(done);
  }

  function push(s: Shape) {
    shapes = [...shapes, s];
    undone = [];
  }
  function undo() {
    if (shapes.length === 0) return;
    undone = [...undone, shapes[shapes.length - 1]];
    shapes = shapes.slice(0, -1);
  }
  function redo() {
    if (undone.length === 0) return;
    shapes = [...shapes, undone[undone.length - 1]];
    undone = undone.slice(0, -1);
  }

  function commitText() {
    if (typing && typing.text.trim()) push({ kind: 'text', color, width, at: typing.at, text: typing.text });
    typing = null;
  }

  function onKey(e: KeyboardEvent) {
    if (typing) {
      if (e.key === 'Escape') typing = null;
      return;
    }
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'z') {
      e.preventDefault();
      if (e.shiftKey) redo();
      else undo();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'y') {
      e.preventDefault();
      redo();
    } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'c' && !window.getSelection()?.toString()) {
      e.preventDefault();
      void copy();
    } else if (e.key === 'Escape') {
      void shotIpc.close();
    }
  }

  /** The finished picture, without the shape still being drawn. */
  function result(): string {
    draft = null;
    paint();
    return canvas!.toDataURL('image/png');
  }

  async function copy() {
    commitText();
    try {
      await shotIpc.copy(result());
      say('Copied. Paste it anywhere.');
    } catch (e) {
      say(String(e));
    }
  }
  async function saveAs() {
    commitText();
    const stamp = new Date().toISOString().slice(0, 19).replace(/[:T]/g, '-');
    const path = await save({ defaultPath: `screenshot-${stamp}.png`, filters: [{ name: 'PNG picture', extensions: ['png'] }] });
    if (!path) return;
    try {
      await shotIpc.save(path, result());
      say('Saved.');
    } catch (e) {
      say(String(e));
    }
  }
  function say(t: string) {
    note = t;
    setTimeout(() => (note = ''), 2600);
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="editor">
  <div class="bar" role="toolbar" aria-label="Tools">
    <div class="tools">
      {#each TOOLS as t (t.id)}
        <button class:on={tool === t.id} title={t.hint} aria-pressed={tool === t.id} onclick={() => (tool = t.id)}>{t.label}</button>
      {/each}
    </div>
    <div class="paint">
      {#each COLORS as c (c)}
        <button class="sw" class:on={color === c} style="background:{c}" aria-label={`Colour ${c}`} aria-pressed={color === c} onclick={() => (color = c)}></button>
      {/each}
      <span class="sep"></span>
      {#each WIDTHS as w, i (w)}
        <button class="wd" class:on={width === w} aria-label={['Thin', 'Medium', 'Thick'][i]} aria-pressed={width === w} onclick={() => (width = w)}><i style="height:{w}px"></i></button>
      {/each}
    </div>
    <div class="acts">
      <button disabled={shapes.length === 0} onclick={undo} title="Ctrl+Z">Undo</button>
      <button disabled={undone.length === 0} onclick={redo} title="Ctrl+Y">Redo</button>
    </div>
  </div>

  <div class="stage">
    {#if ready}
      <div class="wrap">
        <canvas bind:this={canvas} onpointerdown={down} onpointermove={move} onpointerup={up} class:text={tool === 'text'}></canvas>
        {#if typing}
          <input
            bind:this={typer}
            class="typer"
            style="left:{typing.css.x}px; top:{typing.css.y}px; color:{color}"
            bind:value={typing.text}
            placeholder="Type, then Enter"
            aria-label="Text to put on the picture"
            onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); commitText(); } }}
            onblur={commitText}
          />
        {/if}
      </div>
    {/if}
  </div>

  <div class="foot">
    <span class="note" role="status">{note}</span>
    <button onclick={() => void shotIpc.close()}>Close</button>
    <button onclick={() => void saveAs()}>Save…</button>
    <button class="primary" onclick={() => void copy()}>Copy</button>
  </div>
</div>

<style>
  :global(html), :global(body) { margin: 0; height: 100%; overflow: hidden; background: var(--bg, #110d18); color: var(--text, #faf3fc); font-family: var(--font-ui, system-ui, sans-serif); }
  .editor { display: grid; grid-template-rows: auto 1fr auto; height: 100vh; }
  .bar { display: flex; flex-wrap: wrap; gap: 10px 18px; align-items: center; padding: 8px 12px; background: var(--surface-1, #1a1424); border-bottom: 1px solid var(--border, #333); }
  .tools, .paint, .acts { display: flex; gap: 4px; align-items: center; }
  button { border: 1px solid var(--border-strong, #555); background: var(--surface-2, #241c32); color: inherit; border-radius: 8px; padding: 6px 12px; font: inherit; font-size: 13px; font-weight: 600; cursor: pointer; }
  button:hover:not(:disabled) { background: var(--surface-3, #312744); }
  button.on { background: var(--accent, #ffa970); color: var(--on-accent, #220f05); border-color: transparent; }
  button:disabled { opacity: 0.45; cursor: default; }
  button:focus-visible, input:focus-visible { outline: 2px solid var(--focus, #ffb380); outline-offset: 2px; }
  .sw { width: 22px; height: 22px; padding: 0; border-radius: 50%; border: 2px solid var(--border-strong, #555); }
  .sw.on { border-color: var(--text, #fff); box-shadow: 0 0 0 2px var(--accent, #ffa970); }
  .wd { width: 30px; padding: 6px 4px; display: grid; place-items: center; }
  .wd i { display: block; width: 18px; background: currentColor; border-radius: 3px; }
  .wd.on { background: var(--surface-3, #312744); color: inherit; box-shadow: inset 0 0 0 2px var(--accent, #ffa970); }
  .sep { width: 1px; height: 20px; background: var(--border-strong, #555); margin: 0 6px; }
  .stage { overflow: auto; display: grid; place-items: center; padding: 14px; background: repeating-conic-gradient(#00000022 0% 25%, transparent 0% 50%) 50% / 20px 20px; min-height: 0; }
  .wrap { position: relative; line-height: 0; box-shadow: 0 8px 30px rgba(0, 0, 0, 0.5); }
  canvas { max-width: calc(100vw - 30px); max-height: calc(100vh - 150px); cursor: crosshair; touch-action: none; }
  canvas.text { cursor: text; }
  .typer { position: absolute; transform: translateY(-50%); min-width: 160px; padding: 4px 8px; font: 700 18px system-ui, sans-serif; background: rgba(0, 0, 0, 0.6); border: 1px solid #fff; border-radius: 6px; }
  .foot { display: flex; gap: 8px; align-items: center; justify-content: flex-end; padding: 8px 12px; background: var(--surface-1, #1a1424); border-top: 1px solid var(--border, #333); }
  .note { margin-right: auto; color: var(--ok, #7be8bc); font-size: 13px; }
  .primary { background: linear-gradient(135deg, var(--accent, #ffa970), var(--blush, #ff8fb8)); color: var(--on-accent, #220f05); border-color: transparent; font-weight: 700; }
</style>
