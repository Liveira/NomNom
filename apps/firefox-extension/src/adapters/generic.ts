import { createButton, removeButtons } from '../shared/button';
import { send, type DownloadPreset, type Result } from '../shared/messaging';
import { ALL_SITES } from '../shared/sites';
import type { Adapter } from '../shared/run';

const MIN_WIDTH = 200;
const MIN_HEIGHT = 110;
const HIDE_AFTER_MS = 700;
/** Links that point at one video or clip, as opposed to a profile or a search. */
const CONTENT_PATH = /\/(video|videos|watch|reel|reels|clip|clips|v|status|post|pin|gallery|a)\/|^\/\d+\/?$|^\/[\w.-]+\/video\//i;

function siteAllowed(url: URL): boolean {
  return url.protocol === 'https:' && ALL_SITES.some((s) => url.hostname === s || url.hostname.endsWith('.' + s));
}

/** The address of the video under the pointer. Feeds show many videos on one page, so the link
 *  around the player is preferred; on a page of its own the address bar is the answer. */
export function videoPageUrl(video: Element, loc: { href: string }): string | null {
  let node: Element | null = video;
  for (let depth = 0; node && depth < 8; depth++, node = node.parentElement) {
    const links = node.matches('a[href]') ? [node] : [...node.querySelectorAll('a[href]')];
    for (const a of links) {
      try {
        const u = new URL(a.getAttribute('href') ?? '', loc.href);
        if (siteAllowed(u) && CONTENT_PATH.test(u.pathname)) {
          u.hash = '';
          return u.toString();
        }
      } catch {
        /* not a link */
      }
    }
  }
  try {
    const here = new URL(loc.href);
    here.hash = '';
    return siteAllowed(here) ? here.toString() : null;
  } catch {
    return null;
  }
}

export function bigVideoAt(doc: Document, x: number, y: number): HTMLVideoElement | null {
  for (const v of doc.querySelectorAll('video')) {
    const r = v.getBoundingClientRect();
    if (r.width >= MIN_WIDTH && r.height >= MIN_HEIGHT && x >= r.left && x <= r.right && y >= r.top && y <= r.bottom) return v;
  }
  return null;
}

/** One small button that appears in the corner of whichever video the pointer is over. */
export function mountPlayerButton(doc: Document = document, loc: { href: string } = location): () => void {
  let current: HTMLVideoElement | null = null;
  let hideTimer: ReturnType<typeof setTimeout> | undefined;

  const target = () => (current ? videoPageUrl(current, loc) : null);
  const act = (preset?: DownloadPreset): Promise<Result> => {
    const url = target();
    return url ? send({ kind: 'download', url, preset }) : Promise.resolve({ ok: false, message: "Couldn't find this video's link." });
  };

  const holder = doc.createElement('div');
  holder.dataset.nomnomFloat = '';
  holder.className = 'nomnom-player';
  holder.style.cssText = 'position:fixed;z-index:2147483646;display:none';
  holder.appendChild(
    createButton({
      site: 'player',
      compact: true,
      onDownload: () => act(),
      onPreset: (p) => act(p),
      onOpenInApp: () => {
        const url = target();
        return url ? send({ kind: 'open', url }) : Promise.resolve({ ok: false, message: "Couldn't find this video's link." });
      },
    }),
  );
  (doc.body ?? doc.documentElement).appendChild(holder);

  const hide = () => {
    holder.style.display = 'none';
    current = null;
  };
  const menuOpen = () => doc.querySelector('.nomnom-menu') !== null;
  const place = (video: HTMLVideoElement) => {
    const r = video.getBoundingClientRect();
    current = video;
    holder.style.display = 'block';
    const w = holder.offsetWidth || 40;
    holder.style.left = `${Math.max(4, Math.min(r.right - w - 8, (doc.defaultView?.innerWidth ?? 1024) - w - 4))}px`;
    holder.style.top = `${Math.max(4, r.top + 8)}px`;
  };

  let last = 0;
  const onMove = (e: MouseEvent) => {
    const now = Date.now();
    if (now - last < 60) return;
    last = now;
    if (holder.contains(e.target as Node)) {
      clearTimeout(hideTimer);
      return;
    }
    const video = bigVideoAt(doc, e.clientX, e.clientY);
    if (video) {
      clearTimeout(hideTimer);
      hideTimer = undefined;
      place(video);
    } else if (!hideTimer && holder.style.display !== 'none') {
      hideTimer = setTimeout(() => {
        hideTimer = undefined;
        if (!menuOpen()) hide();
      }, HIDE_AFTER_MS);
    }
  };
  const onScroll = () => {
    if (current && !menuOpen()) place(current);
  };
  doc.addEventListener('mousemove', onMove, true);
  doc.addEventListener('scroll', onScroll, true);
  return () => {
    clearTimeout(hideTimer);
    doc.removeEventListener('mousemove', onMove, true);
    doc.removeEventListener('scroll', onScroll, true);
    holder.remove();
    removeButtons(doc);
  };
}

export const player: Adapter = { mount: () => mountPlayerButton() };
