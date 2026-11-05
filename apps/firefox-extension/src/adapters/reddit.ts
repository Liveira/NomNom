import { createButton, hasButton, removeButtons } from '../shared/button';
import { send, type Result } from '../shared/messaging';
import { watch } from '../shared/observe';
import type { Adapter } from '../shared/run';

const SEEN = 'data-nomnom-seen';

/** Preview links carry size parameters; the original lives at i.redd.it with the same file name. */
export function originalImage(src: string): string | null {
  try {
    const u = new URL(src, 'https://www.reddit.com');
    if (!['preview.redd.it', 'i.redd.it', 'external-preview.redd.it'].includes(u.hostname)) return null;
    const file = u.pathname.split('/').filter(Boolean).pop();
    return file && /\.(jpe?g|png|gif|webp)$/i.test(file) ? `https://i.redd.it/${file}` : null;
  } catch {
    return null;
  }
}

export function postLink(post: Element): string | null {
  const permalink = post.getAttribute('permalink');
  return permalink && permalink.startsWith('/r/') ? `https://www.reddit.com${permalink}` : null;
}

export function postImages(post: Element): string[] {
  const out = new Set<string>();
  const href = post.getAttribute('content-href');
  const direct = href ? originalImage(href) : null;
  if (direct) out.add(direct);
  if (out.size === 0) {
    for (const img of post.querySelectorAll('img')) {
      const o = originalImage(img.getAttribute('src') ?? '');
      if (o) out.add(o);
    }
  }
  return [...out];
}

export function hasMedia(post: Element): boolean {
  const type = post.getAttribute('post-type');
  return type === 'video' || type === 'image' || type === 'gallery';
}

function download(post: Element): Promise<Result> {
  const link = postLink(post);
  if (!link) return Promise.resolve({ ok: false, message: "Couldn't find this post's link." });
  const type = post.getAttribute('post-type');
  if (type === 'video') return send({ kind: 'download', url: link });
  const images = postImages(post);
  if (images.length > 0) return send({ kind: 'media', source: link, media: images });
  return Promise.resolve({ ok: false, message: 'This post has no media NomNom can download.' });
}

/** Reddit draws its action bar inside a shadow root, so the button is placed over the post's
 *  corner instead of inside the bar. */
export function mountReddit(doc: Document = document): () => void {
  const scan = () => {
    for (const post of doc.querySelectorAll(`shreddit-post:not([${SEEN}])`)) {
      if (!hasMedia(post) || hasButton(post)) continue;
      const host = post as HTMLElement;
      if (getComputedStyle(host).position === 'static') host.style.position = 'relative';
      const holder = doc.createElement('span');
      holder.className = 'nomnom-overlay';
      holder.appendChild(createButton({ site: 'reddit', compact: true, onDownload: () => download(post) }));
      post.appendChild(holder);
      post.setAttribute(SEEN, '');
    }
  };
  const stop = watch(doc.body, scan);
  return () => {
    stop();
    doc.querySelectorAll('.nomnom-overlay').forEach((el) => el.remove());
    removeButtons(doc);
    doc.querySelectorAll(`[${SEEN}]`).forEach((a) => a.removeAttribute(SEEN));
  };
}

export const reddit: Adapter = { mount: () => mountReddit() };
