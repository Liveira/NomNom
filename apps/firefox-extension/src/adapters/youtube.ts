import { createButton, hasButton, removeButtons } from '../shared/button';
import { send, type DownloadPreset } from '../shared/messaging';
import { watch } from '../shared/observe';
import type { Adapter } from '../shared/run';

/** The canonical link for what is playing, or null when the page isn't a video. Playlist and
 *  tracking parameters are dropped so the link is stable. */
export function videoUrl(loc: { pathname: string; search: string }): string | null {
  const id = (s: string | null) => (s && /^[\w-]{6,20}$/.test(s) ? s : null);
  if (loc.pathname === '/watch') {
    const v = id(new URLSearchParams(loc.search).get('v'));
    return v ? `https://www.youtube.com/watch?v=${v}` : null;
  }
  const shorts = /^\/shorts\/([\w-]{6,20})\/?$/.exec(loc.pathname);
  return shorts ? `https://www.youtube.com/shorts/${shorts[1]}` : null;
}

/** Where the Like and Share buttons live. YouTube changes these; several are tried in order. */
const WATCH_CONTAINERS = [
  'ytd-watch-metadata #top-level-buttons-computed',
  '#above-the-fold #top-level-buttons-computed',
  '#actions #top-level-buttons-computed',
  'ytd-menu-renderer #top-level-buttons-computed',
];
const SHORTS_CONTAINERS = [
  'ytd-reel-video-renderer[is-active] #actions',
  'ytd-reel-video-renderer[is-active] reel-action-bar-view-model',
];

function first(root: ParentNode, selectors: string[]): Element | null {
  for (const s of selectors) {
    const el = root.querySelector(s);
    if (el) return el;
  }
  return null;
}

export function mountYoutube(doc: Document = document, loc: Location = location): () => void {
  const current = () => videoUrl(loc);
  const act = (preset?: DownloadPreset) => {
    const url = current(); // read at click time, so it is always the video on screen
    return url ? send({ kind: 'download', url, preset }) : Promise.resolve({ ok: false, message: 'Open a video first.' });
  };

  const apply = () => {
    const url = current();
    if (!url) {
      removeButtons(doc);
      return;
    }
    const shorts = url.includes('/shorts/');
    const container = first(doc, shorts ? SHORTS_CONTAINERS : WATCH_CONTAINERS);
    if (!container) return;
    if (hasButton(container)) return;
    removeButtons(doc); // a button left in a replaced container must not linger
    const button = createButton({
      site: 'youtube',
      compact: shorts,
      onDownload: () => act(),
      onPreset: (p) => act(p),
      onOpenInApp: () => {
        const u = current();
        return u ? send({ kind: 'open', url: u }) : Promise.resolve({ ok: false, message: 'Open a video first.' });
      },
    });
    if (shorts) container.prepend(button);
    else container.appendChild(button);
  };

  const stop = watch(doc.body, apply);
  return () => {
    stop();
    removeButtons(doc);
  };
}

export const youtube: Adapter = {
  mount: () => mountYoutube(),
  navEvents: ['yt-navigate-finish', 'yt-page-data-updated'],
};
