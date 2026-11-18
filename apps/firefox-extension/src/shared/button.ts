import type { DownloadPreset, Result } from './messaging';
import { ensureStyles } from './styles';

export type Site = 'youtube' | 'x' | 'instagram' | 'reddit' | 'player';
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

/** Puts a menu or a message in its own layer on the page, next to `anchor`. Posts and toolbars
 *  often clip what sticks out of them, so these are never placed inside the page's own layout. */
export function float(doc: Document, el: HTMLElement, anchor: HTMLElement): void {
  el.dataset.nomnomFloat = '';
  el.style.position = 'fixed';
  el.style.visibility = 'hidden';
  (doc.body ?? doc.documentElement).appendChild(el);
  const view = doc.defaultView;
  const a = anchor.getBoundingClientRect();
  const w = el.offsetWidth;
  const h = el.offsetHeight;
  const vw = view?.innerWidth ?? 1024;
  const vh = view?.innerHeight ?? 768;
  const gap = 6;
  // Below the button when it fits, otherwise above it; lined up with the button's right edge,
  // and always inside the window.
  let top = a.bottom + gap;
  if (top + h > vh - 8 && a.top - gap - h >= 8) top = a.top - gap - h;
  let left = a.right - w;
  left = Math.max(8, Math.min(left, vw - w - 8));
  el.style.top = `${Math.max(8, top)}px`;
  el.style.left = `${left}px`;
  el.style.visibility = '';
}

/** Does the page look dark? Sites like X and Reddit have their own dark mode that doesn't follow the
 *  system's, so the colour of the page itself is what counts. */
export function pageIsDark(doc: Document): boolean {
  const view = doc.defaultView;
  for (const el of [doc.body, doc.documentElement]) {
    if (!el || !view) continue;
    const m = /rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?/.exec(view.getComputedStyle(el).backgroundColor);
    if (!m || (m[4] !== undefined && parseFloat(m[4]) === 0)) continue;
    const [r, g, b] = [+m[1], +m[2], +m[3]];
    return 0.2126 * r + 0.7152 * g + 0.0722 * b < 140;
  }
  return view?.matchMedia?.('(prefers-color-scheme: dark)').matches ?? false;
}

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
    toast.dataset.scheme = pageIsDark(doc) ? 'dark' : 'light';
    float(doc, toast, wrap);
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

  const plainClick = (e: Event) => {
    swallow(e);
    void run(opts.onDownload);
  };
  main.addEventListener('click', plainClick);

  if (opts.onPreset) {
    // Wide buttons (YouTube) have a small arrow for the menu. Icon-only buttons have no room for
    // one, so a click on the icon itself opens the menu.
    let opener: HTMLButtonElement = main;
    if (!opts.compact) {
      const more = doc.createElement('button');
      more.type = 'button';
      more.className = 'nomnom-more';
      more.setAttribute('aria-label', 'More download options');
      more.appendChild(caretIcon(doc));
      wrap.appendChild(more);
      opener = more;
    }
    opener.setAttribute('aria-haspopup', 'menu');
    opener.setAttribute('aria-expanded', 'false');
    if (opts.compact) {
      main.title = 'Download with NomNom: choose video or audio';
      main.setAttribute('aria-label', 'Download with NomNom, show options');
    }

    const onOutside = (e: Event) => {
      const t = e.target as Node;
      if (!wrap.contains(t) && !menu?.contains(t)) close();
    };
    const onMove = () => close();
    function close() {
      if (!menu) return;
      menu.remove();
      menu = null;
      opener.setAttribute('aria-expanded', 'false');
      doc.removeEventListener('click', onOutside, true);
      doc.defaultView?.removeEventListener('scroll', onMove, true);
      doc.defaultView?.removeEventListener('resize', onMove);
    }
    const open = () => {
      toast?.remove();
      menu = doc.createElement('div');
      menu.className = 'nomnom-menu';
      menu.setAttribute('role', 'menu');
      menu.dataset.scheme = pageIsDark(doc) ? 'dark' : 'light';
      const items: { label: string; go: () => Promise<Result> }[] = CHOICES.map((c) => ({
        label: c.label,
        go: () => (c.preset.kind === 'video' && opts.compact ? opts.onDownload() : opts.onPreset!(c.preset)),
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
          opener.focus();
        } else return;
        e.preventDefault();
        e.stopPropagation();
      });
      float(doc, menu, wrap);
      opener.setAttribute('aria-expanded', 'true');
      doc.addEventListener('click', onOutside, true);
      doc.defaultView?.addEventListener('scroll', onMove, true);
      doc.defaultView?.addEventListener('resize', onMove);
      menu.querySelector('button')?.focus();
    };
    const toggle = (e: Event) => {
      swallow(e);
      if (menu) close();
      else open();
    };
    opener.addEventListener('click', toggle);
    if (opts.compact) {
      // The icon opens the menu, so the plain-click download below must not also run.
      main.removeEventListener('click', plainClick);
    }
  }
  return wrap;
}

export function hasButton(container: ParentNode): boolean {
  return container.querySelector(`[${MARK}]`) !== null;
}

export function removeButtons(root: ParentNode = document): void {
  root.querySelectorAll(`[${MARK}], [data-nomnom-float]`).forEach((el) => el.remove());
}
