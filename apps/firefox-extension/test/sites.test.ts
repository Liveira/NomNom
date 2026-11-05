import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createButton, FLASH_MS } from '../src/shared/button';
import { bestImage, mountInstagram, postImages, postUrl } from '../src/adapters/instagram';
import { mountReddit, originalImage, postImages as redditImages } from '../src/adapters/reddit';
import { count, installBrowser, mountPoint, setPath, settle } from './helpers';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('Instagram', () => {
  const post = (code: string, media: string) => `
    <article>
      <header><img alt="alice's profile picture" src="https://scontent.cdninstagram.com/avatar.jpg"></header>
      <a href="/alice/p/${code}/"><time></time></a>
      ${media}
      <section><div role="button"><svg aria-label="Like"></svg></div><div role="button"><svg aria-label="Share"></svg></div></section>
    </article>`;
  const img = '<img srcset="https://scontent.cdninstagram.com/s640.jpg 640w, https://scontent.cdninstagram.com/s1080.jpg 1080w" src="https://scontent.cdninstagram.com/s320.jpg" alt="Photo by alice">';

  it('finds the post link and ignores the profile picture', () => {
    mountPoint(post('AbC123', img));
    const a = document.querySelector('article')!;
    expect(postUrl(a, { pathname: '/' })).toBe('https://www.instagram.com/p/AbC123/');
    expect(postImages(a)).toEqual(['https://scontent.cdninstagram.com/s1080.jpg']);
  });

  it('picks the largest picture', () => {
    mountPoint(img);
    expect(bestImage(document.querySelector('img')!)).toBe('https://scontent.cdninstagram.com/s1080.jpg');
  });

  it('adds one button after Share on each post, and never doubles', async () => {
    installBrowser();
    const body = mountPoint(post('A1', img) + post('B2', img));
    const stop = mountInstagram();
    for (let i = 0; i < 10; i++) {
      body.appendChild(document.createElement('div'));
      await settle(50);
    }
    expect(count('[data-nomnom]')).toBe(2);
    const first = document.querySelector('article section')!;
    expect(first.lastElementChild!.hasAttribute('data-nomnom')).toBe(true);
    stop();
  });

  it('sends pictures for a photo post and the link for a reel', async () => {
    const fake = installBrowser();
    mountPoint(post('PHOTO1', img) + post('REEL1', '<video src="x"></video>').replace('/p/REEL1/', '/reel/REEL1/'));
    const stop = mountInstagram();
    await settle();
    const [photoBtn, reelBtn] = [...document.querySelectorAll<HTMLButtonElement>('.nomnom-main')];
    photoBtn.click();
    await settle(10);
    expect(fake.sendMessage.mock.calls[0][0]).toMatchObject({ kind: 'media', source: 'https://www.instagram.com/p/PHOTO1/' });
    reelBtn.click();
    await settle(10);
    expect(fake.sendMessage.mock.calls[1][0]).toMatchObject({ kind: 'download', url: 'https://www.instagram.com/reel/REEL1/' });
    stop();
  });

  it('uses the address bar on a post page', () => {
    mountPoint('<article></article>');
    setPath('/p/XyZ9/');
    expect(postUrl(document.querySelector('article')!)).toBe('https://www.instagram.com/p/XyZ9/');
  });
});

describe('Reddit', () => {
  const post = (type: string, extra = '', body = '') =>
    `<shreddit-post permalink="/r/cats/comments/abc/title/" post-type="${type}" ${extra}>${body}</shreddit-post>`;

  it('turns preview links into the original picture', () => {
    expect(originalImage('https://preview.redd.it/abc123.jpg?width=640&auto=webp&s=x')).toBe('https://i.redd.it/abc123.jpg');
    expect(originalImage('https://i.redd.it/pic.png')).toBe('https://i.redd.it/pic.png');
    expect(originalImage('https://evil.example/abc.jpg')).toBeNull();
    expect(originalImage('https://i.redd.it/noextension')).toBeNull();
  });

  it('collects gallery pictures once each', () => {
    mountPoint(post('gallery', '', '<img src="https://preview.redd.it/a.jpg?width=1"><img src="https://preview.redd.it/a.jpg?width=2"><img src="https://preview.redd.it/b.png">'));
    expect(redditImages(document.querySelector('shreddit-post')!)).toEqual(['https://i.redd.it/a.jpg', 'https://i.redd.it/b.png']);
  });

  it('adds a button only to posts with media, once', async () => {
    installBrowser();
    mountPoint(post('image', 'content-href="https://i.redd.it/x.jpg"') + post('text') + post('video'));
    const stop = mountReddit();
    for (let i = 0; i < 5; i++) {
      document.body.appendChild(document.createElement('div'));
      await settle(50);
    }
    const posts = [...document.querySelectorAll('shreddit-post')];
    expect(posts.map((p) => count('[data-nomnom]', p))).toEqual([1, 0, 1]);
    stop();
    expect(count('[data-nomnom]')).toBe(0);
  });

  it('sends the picture for an image post and the post link for a video', async () => {
    const fake = installBrowser();
    mountPoint(post('image', 'content-href="https://i.redd.it/x.jpg"') + post('video'));
    const stop = mountReddit();
    await settle();
    const [imageBtn, videoBtn] = [...document.querySelectorAll<HTMLButtonElement>('.nomnom-main')];
    imageBtn.click();
    await settle(10);
    expect(fake.sendMessage.mock.calls[0][0]).toEqual({ kind: 'media', source: 'https://www.reddit.com/r/cats/comments/abc/title/', media: ['https://i.redd.it/x.jpg'] });
    videoBtn.click();
    await settle(10);
    expect(fake.sendMessage.mock.calls[1][0]).toEqual({ kind: 'download', url: 'https://www.reddit.com/r/cats/comments/abc/title/' });
    stop();
  });
});

describe('the button itself', () => {
  const mk = (onDownload: () => Promise<{ ok: boolean; message: string }>) => {
    document.body.innerHTML = '';
    const b = createButton({ site: 'x', onDownload, onPreset: async () => ({ ok: true, message: 'x' }) });
    document.body.appendChild(b);
    return b;
  };

  it('shows sending, then success, then goes back to normal', async () => {
    let done!: (v: { ok: boolean; message: string }) => void;
    const b = mk(() => new Promise((r) => (done = r)));
    (b.querySelector('.nomnom-main') as HTMLButtonElement).click();
    expect(b.dataset.state).toBe('sending');
    expect(b.getAttribute('aria-busy')).toBe('true');
    done({ ok: true, message: 'Added to NomNom.' });
    await settle(1);
    expect(b.dataset.state).toBe('ok');
    expect(b.querySelector('[role="status"]')!.textContent).toBe('Added to NomNom.');
    await settle(FLASH_MS + 50);
    expect(b.dataset.state).toBe('idle');
    expect(b.querySelector('[role="status"]')).toBeNull();
  });

  it('shows the error text when something goes wrong', async () => {
    const b = mk(async () => ({ ok: false, message: "NomNom isn't running." }));
    (b.querySelector('.nomnom-main') as HTMLButtonElement).click();
    await settle(1);
    expect(b.dataset.state).toBe('error');
    expect(b.textContent).toContain("NomNom isn't running.");
  });

  it('copes with the request failing outright', async () => {
    const b = mk(() => Promise.reject(new Error('boom')));
    (b.querySelector('.nomnom-main') as HTMLButtonElement).click();
    await settle(1);
    expect(b.dataset.state).toBe('error');
  });

  it('ignores a second click while the first is still being sent', () => {
    const fn = vi.fn(() => new Promise<{ ok: boolean; message: string }>(() => {}));
    const b = mk(fn);
    const main = b.querySelector('.nomnom-main') as HTMLButtonElement;
    main.click();
    main.click();
    main.click();
    expect(fn).toHaveBeenCalledTimes(1);
  });

  it('keeps the click away from the page underneath', () => {
    const b = mk(async () => ({ ok: true, message: 'x' }));
    const page = vi.fn();
    document.body.addEventListener('click', page);
    (b.querySelector('.nomnom-main') as HTMLButtonElement).click();
    expect(page).not.toHaveBeenCalled();
  });

  it('the menu opens, moves with the arrow keys, and closes with Escape', () => {
    const b = mk(async () => ({ ok: true, message: 'x' }));
    const more = b.querySelector('.nomnom-more') as HTMLButtonElement;
    more.click();
    expect(more.getAttribute('aria-expanded')).toBe('true');
    const items = [...b.querySelectorAll<HTMLButtonElement>('[role="menuitem"]')];
    expect(document.activeElement).toBe(items[0]);
    items[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowDown', bubbles: true }));
    expect(document.activeElement).toBe(items[1]);
    items[1].dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(b.querySelector('[role="menu"]')).toBeNull();
    expect(more.getAttribute('aria-expanded')).toBe('false');
  });

  it('clicking elsewhere closes the menu', () => {
    const b = mk(async () => ({ ok: true, message: 'x' }));
    (b.querySelector('.nomnom-more') as HTMLButtonElement).click();
    document.body.click();
    expect(b.querySelector('[role="menu"]')).toBeNull();
  });
});
