import { createButton, hasButton, removeButtons } from '../shared/button';
import { send, type Result } from '../shared/messaging';
import { watch } from '../shared/observe';
import type { Adapter } from '../shared/run';

const SEEN = 'data-nomnom-seen';

/** The link to a post, taken from a link inside it or, on a post page, from the address bar. */
export function postUrl(article: Element, loc: { pathname: string } = location): string | null {
  const pick = (path: string) => {
    const m = /^\/(?:[\w.]+\/)?(p|reel|tv)\/([\w-]+)/.exec(path);
    return m ? `https://www.instagram.com/${m[1]}/${m[2]}/` : null;
  };
  for (const a of article.querySelectorAll('a[href]')) {
    const url = pick(a.getAttribute('href') ?? '');
    if (url) return url;
  }
  return pick(loc.pathname);
}

/** The largest candidate in an `srcset`, or the plain `src`. */
export function bestImage(img: Element): string | null {
  const srcset = img.getAttribute('srcset');
  if (srcset) {
    const best = srcset
      .split(',')
      .map((part) => part.trim().split(/\s+/))
      .map(([url, w]) => ({ url, w: parseInt(w ?? '0', 10) || 0 }))
      .sort((a, b) => b.w - a.w)[0];
    if (best?.url) return best.url;
  }
  return img.getAttribute('src');
}

export function postImages(article: Element): string[] {
  const out = new Set<string>();
  for (const img of article.querySelectorAll('img')) {
    if (img.closest('header')) continue; // the author's profile picture
    if (/profile picture/i.test(img.getAttribute('alt') ?? '')) continue;
    const url = bestImage(img);
    if (!url) continue;
    try {
      const host = new URL(url).hostname;
      if (host.endsWith('.cdninstagram.com') || host.endsWith('.fbcdn.net')) out.add(url);
    } catch {
      /* ignore anything that isn't a link */
    }
  }
  return [...out];
}

function actionBar(article: Element): Element | null {
  const marker = article.querySelector('svg[aria-label="Share"], svg[aria-label="Share Post"], svg[aria-label="Like"], svg[aria-label="Unlike"]');
  return marker?.closest('section') ?? null;
}

function download(article: Element): Promise<Result> {
  const url = postUrl(article);
  if (!url) return Promise.resolve({ ok: false, message: "Couldn't find this post's link." });
  if (article.querySelector('video') || url.includes('/reel/')) return send({ kind: 'download', url });
  const images = postImages(article);
  if (images.length > 0) return send({ kind: 'media', source: url, media: images });
  return Promise.resolve({ ok: false, message: 'No pictures or video in this post.' });
}

export function mountInstagram(doc: Document = document): () => void {
  const scan = () => {
    for (const article of doc.querySelectorAll(`article:not([${SEEN}])`)) {
      const bar = actionBar(article);
      if (!bar || hasButton(bar)) continue;
      const share = bar.querySelector('svg[aria-label="Share"], svg[aria-label="Share Post"]')?.closest('div[role="button"], button');
      const button = createButton({ site: 'instagram', compact: true, onDownload: () => download(article) });
      if (share) share.insertAdjacentElement('afterend', button);
      else bar.appendChild(button);
      article.setAttribute(SEEN, '');
    }
  };
  const stop = watch(doc.body, scan);
  return () => {
    stop();
    removeButtons(doc);
    doc.querySelectorAll(`[${SEEN}]`).forEach((a) => a.removeAttribute(SEEN));
  };
}

export const instagram: Adapter = { mount: () => mountInstagram() };
