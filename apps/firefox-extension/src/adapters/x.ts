import { createButton, hasButton, removeButtons } from '../shared/button';
import { send, type DownloadPreset, type Result } from '../shared/messaging';
import { watch } from '../shared/observe';
import type { Adapter } from '../shared/run';

const SEEN = 'data-nomnom-seen';
const MEDIA = '[data-testid="videoPlayer"], [data-testid="tweetPhoto"]';

/** A quoted post is shown inside the post that quotes it. Its media and its link belong to the
 *  other post, and must never be taken for the outer one. */
function inQuote(el: Element, article: Element): boolean {
  const card = el.closest('div[role="link"]');
  return card !== null && card !== article && article.contains(card);
}

function own<T extends Element>(article: Element, selector: string): T[] {
  return [...article.querySelectorAll<T>(selector)].filter((el) => !inQuote(el, article));
}

/** The link to this post, from its own timestamp. */
export function tweetUrl(article: Element): string | null {
  for (const time of own<HTMLTimeElement>(article, 'time')) {
    const href = time.closest('a')?.getAttribute('href') ?? '';
    const m = /^\/([A-Za-z0-9_]{1,15})\/status\/(\d+)/.exec(href);
    if (m) return `https://x.com/${m[1]}/status/${m[2]}`;
  }
  return null;
}

export function hasOwnVideo(article: Element): boolean {
  return own(article, '[data-testid="videoPlayer"]').length > 0;
}

/** Full-size picture links for this post's own photos, in order, without duplicates. */
export function photoUrls(article: Element): string[] {
  const seen = new Set<string>();
  for (const photo of own(article, '[data-testid="tweetPhoto"]')) {
    for (const img of photo.querySelectorAll('img')) {
      let url: URL;
      try {
        url = new URL(img.getAttribute('src') ?? '');
      } catch {
        continue;
      }
      if (url.hostname !== 'pbs.twimg.com' || !url.pathname.startsWith('/media/')) continue;
      url.searchParams.set('name', 'orig');
      seen.add(url.toString());
    }
  }
  return [...seen];
}

export function hasOwnMedia(article: Element): boolean {
  return own(article, MEDIA).length > 0;
}

function actionRow(article: Element): Element | null {
  const rows = own(article, '[role="group"]').filter((g) => g.querySelector('[data-testid="reply"], [data-testid="like"], [data-testid="unlike"]'));
  return rows.at(-1) ?? null;
}

function download(article: Element, preset?: DownloadPreset): Promise<Result> {
  const url = tweetUrl(article);
  if (!url) return Promise.resolve({ ok: false, message: "Couldn't find this post's link." });
  if (hasOwnVideo(article)) return send({ kind: 'download', url, preset });
  const photos = photoUrls(article);
  if (photos.length > 0) return send({ kind: 'media', source: url, media: photos });
  return Promise.resolve({ ok: false, message: 'No pictures or video in this post.' });
}

export function mountX(doc: Document = document): () => void {
  const scan = () => {
    for (const article of doc.querySelectorAll(`article[data-testid="tweet"]:not([${SEEN}])`)) {
      if (!hasOwnMedia(article)) continue;
      const row = actionRow(article);
      if (!row || hasButton(row)) continue;
      row.appendChild(
        createButton({
          site: 'x',
          compact: true,
          onDownload: () => download(article),
          onPreset: undefined,
        }),
      );
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

export const x: Adapter = { mount: () => mountX() };
