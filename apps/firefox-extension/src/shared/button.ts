import type { DownloadPreset, Result } from './messaging';
import { ensureStyles } from './styles';

export type Site = 'youtube' | 'x' | 'instagram' | 'reddit';
export const MARK = 'data-nomnom';
export const FLASH_MS = 1800;

export interface ButtonOptions {
  site: Site;
  /** Icon only (X, Instagram, Reddit, Shorts). YouTube's watch page also shows a word. */
  compact?: boolean;
  /** Runs for a plain click. */
  onDownload: () => Promise<Result>;
  /** Runs for a choice from the small menu. Omit to hide the menu. */
  onPreset?: (preset: DownloadPreset) => Promise<Result>;
  onOpenInApp?: () => Promise<Result>;
}

const SVG_NS = 'http://www.w3.org/2000/svg';

/** An icon built from DOM nodes (never from an HTML string). */
function icon(doc: Document, viewBox: string, path: string, width: string): SVGElement {
  const svg = doc.createElementNS(SVG_NS, 'svg');
  svg.setAttribute('viewBox', viewBox);
  svg.setAttribute('fill', 'none');
  svg.setAttribute('stroke', 'currentColor');
  svg.setAttribute('stroke-width', width);
  svg.setAttribute('stroke-linecap', 'round');
  svg.setAttribute('stroke-linejoin', 'round');
  svg.setAttribute('aria-hidden', 'true');
  const p = doc.createElementNS(SVG_NS, 'path');
  p.setAttribute('d', path);
  svg.appendChild(p);
  return svg;
}
const downIcon = (doc: Document) => icon(doc, '0 0 24 24', 'M12 4v11m0 0-4.5-4.5M12 15l4.5-4.5M5 20h14', '2.2');
const caretIcon = (doc: Document) => icon(doc, '0 0 12 12', 'm2.5 4.5 3.5 3.5 3.5-3.5', '1.8');

const CHOICES: { label: string; preset: DownloadPreset }[] = [
  { label: 'Video', preset: { kind: 'video' } },
  { label: 'Audio · MP3', preset: { kind: 'audio', format: 'mp3' } },
  { label: 'Audio · M4A', preset: { kind: 'audio', format: 'm4a' } },
];

export function createButton(opts: ButtonOptions, doc: Document = document): HTMLElement {
  ensureStyles(doc);
  const wrap = doc.createElement('span');
  wrap.className = 'nomnom-wrap';
  wrap.setAttribute(MARK, opts.site);
  wrap.dataset.site = opts.site;
  wrap.dataset.state = 'idle';
  if (opts.compact) wrap.dataset.compact = '';

  const main = doc.createElement('button');
  main.type = 'button';
  main.className = 'nomnom-main';
  main.setAttribute('aria-label', 'Download with NomNom');
  main.title = 'Download with NomNom';
  main.appendChild(downIcon(doc));
  if (!opts.compact) {
    const text = doc.createElement('span');
    text.className = 'nomnom-text';
    text.textContent = 'Download';
    main.appendChild(text);
  }
  wrap.appendChild(main);

  let menu: HTMLDivElement | null = null;
  let toast: HTMLDivElement | null = null;
  let timer: ReturnType<typeof setTimeout> | undefined;

  const say = (text: string) => {
    toast?.remove();
    toast = doc.createElement('div');
    toast.className = 'nomnom-toast';
    toast.setAttribute('role', 'status');
    toast.textContent = text;
    wrap.appendChild(toast);
  };

  const finish = (r: Result) => {
    wrap.dataset.state = r.ok ? 'ok' : 'error';
    wrap.removeAttribute('aria-busy');
    main.disabled = false;
    say(r.message);
    clearTimeout(timer);
    timer = setTimeout(() => {
      wrap.dataset.state = 'idle';
      toast?.remove();
      toast = null;
    }, FLASH_MS);
  };

  const run = async (action: () => Promise<Result>) => {
    if (wrap.dataset.state === 'sending') return;
    wrap.dataset.state = 'sending';
    wrap.setAttribute('aria-busy', 'true');
    main.disabled = true;
    toast?.remove();
    try {
      finish(await action());
    } catch {
      finish({ ok: false, message: "Couldn't send this to NomNom." });
    }
  };

  // The page has its own click handlers (opening a post, starting a video). Keep them out.
  const swallow = (e: Event) => {
    e.preventDefault();
    e.stopPropagation();
  };

  main.addEventListener('click', (e) => {
    swallow(e);
    void run(opts.onDownload);
  });

  if (opts.onPreset && !opts.compact) {
    const more = doc.createElement('button');
    more.type = 'button';
    more.className = 'nomnom-more';
    more.setAttribute('aria-label', 'More download options');
    more.setAttribute('aria-haspopup', 'menu');
    more.setAttribute('aria-expanded', 'false');
    more.appendChild(caretIcon(doc));
    wrap.appendChild(more);

    const onOutside = (e: Event) => {
      if (!wrap.contains(e.target as Node)) close();
    };
    function close() {
      if (!menu) return;
      menu.remove();
      menu = null;
      more.setAttribute('aria-expanded', 'false');
      doc.removeEventListener('click', onOutside, true);
    }
    const open = () => {
      menu = doc.createElement('div');
      menu.className = 'nomnom-menu';
      menu.setAttribute('role', 'menu');
      const items: { label: string; go: () => Promise<Result> }[] = CHOICES.map((c) => ({
        label: c.label,
        go: () => opts.onPreset!(c.preset),
      }));
      if (opts.onOpenInApp) items.push({ label: 'Choose in NomNom…', go: opts.onOpenInApp });
      for (const item of items) {
        const b = doc.createElement('button');
        b.type = 'button';
        b.setAttribute('role', 'menuitem');
        b.textContent = item.label;
        b.addEventListener('click', (e) => {
          swallow(e);
          close();
          void run(item.go);
        });
        menu.appendChild(b);
      }
      menu.addEventListener('keydown', (e) => {
        const list = [...menu!.querySelectorAll('button')];
        const i = list.indexOf(doc.activeElement as HTMLButtonElement);
        if (e.key === 'ArrowDown') list[(i + 1) % list.length].focus();
        else if (e.key === 'ArrowUp') list[(i - 1 + list.length) % list.length].focus();
        else if (e.key === 'Escape') {
          close();
          more.focus();
        } else return;
        e.preventDefault();
        e.stopPropagation();
      });
      wrap.appendChild(menu);
      more.setAttribute('aria-expanded', 'true');
      doc.addEventListener('click', onOutside, true);
      menu.querySelector('button')?.focus();
    };
    more.addEventListener('click', (e) => {
      swallow(e);
      if (menu) close();
      else open();
    });
  }
  return wrap;
}

export function hasButton(container: ParentNode): boolean {
  return container.querySelector(`[${MARK}]`) !== null;
}

export function removeButtons(root: ParentNode = document): void {
  root.querySelectorAll(`[${MARK}]`).forEach((el) => el.remove());
}
