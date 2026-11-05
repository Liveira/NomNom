import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mountYoutube, videoUrl, youtube } from '../src/adapters/youtube';
import { run } from '../src/shared/run';
import { count, installBrowser, mountPoint, setPath, settle } from './helpers';

const WATCH = `
  <ytd-watch-metadata>
    <div id="actions"><div id="top-level-buttons-computed">
      <button id="like">Like</button><button id="share">Share</button>
    </div></div>
  </ytd-watch-metadata>`;

const SHORTS = `
  <ytd-reel-video-renderer is-active>
    <div id="actions"><button id="like">Like</button></div>
  </ytd-reel-video-renderer>
  <ytd-reel-video-renderer><div id="actions"><button>Other reel</button></div></ytd-reel-video-renderer>`;

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('video links', () => {
  it('keeps only the video id from a watch page', () => {
    expect(videoUrl({ pathname: '/watch', search: '?v=dQw4w9WgXcQ&list=PL1&index=3&t=42s' })).toBe(
      'https://www.youtube.com/watch?v=dQw4w9WgXcQ',
    );
  });
  it('understands Shorts', () => {
    expect(videoUrl({ pathname: '/shorts/abcDEF12345', search: '' })).toBe('https://www.youtube.com/shorts/abcDEF12345');
  });
  it('returns nothing for pages that are not a video, or have a broken id', () => {
    for (const [p, s] of [['/', ''], ['/feed/subscriptions', ''], ['/watch', ''], ['/watch', '?v='], ['/watch', '?v=<script>'], ['/shorts/', '']]) {
      expect(videoUrl({ pathname: p, search: s }), `${p}${s}`).toBeNull();
    }
  });
});

describe('the watch page button', () => {
  it('is added next to Like and Share, once', async () => {
    installBrowser();
    setPath('/watch?v=abc123XYZ_-');
    mountPoint(WATCH);
    const stop = mountYoutube();
    await settle();
    expect(count('[data-nomnom]')).toBe(1);
    expect(document.querySelector('#top-level-buttons-computed')!.lastElementChild!.hasAttribute('data-nomnom')).toBe(true);
    stop();
  });

  it('never doubles up, however many times the page changes', async () => {
    installBrowser();
    setPath('/watch?v=abc123XYZ_-');
    const body = mountPoint(WATCH);
    const stop = mountYoutube();
    for (let i = 0; i < 25; i++) {
      body.appendChild(document.createElement('div')); // unrelated churn
      await settle(60);
    }
    await settle();
    expect(count('[data-nomnom]')).toBe(1);
    stop();
  });

  it('survives YouTube replacing the whole button row when you go to another video', async () => {
    installBrowser();
    setPath('/watch?v=firstVideo01');
    const body = mountPoint(WATCH);
    const stop = mountYoutube();
    await settle();
    body.innerHTML = WATCH; // navigation: a fresh row without our button
    await settle();
    expect(count('[data-nomnom]')).toBe(1);
    stop();
  });

  it('takes the button away on pages that are not a video', async () => {
    installBrowser();
    setPath('/watch?v=abc123XYZ_-');
    mountPoint(WATCH);
    const stop = mountYoutube();
    await settle();
    setPath('/feed/subscriptions');
    document.body.appendChild(document.createElement('p'));
    await settle();
    expect(count('[data-nomnom]')).toBe(0);
    stop();
  });

  it('sends the video that is on screen at the moment of the click, not the one it was added for', async () => {
    const fake = installBrowser();
    setPath('/watch?v=oldVideo001');
    mountPoint(WATCH);
    const stop = mountYoutube();
    await settle();
    setPath('/watch?v=newVideo002&list=PLxyz');
    (document.querySelector('.nomnom-main') as HTMLButtonElement).click();
    await settle(10);
    expect(fake.sendMessage).toHaveBeenCalledWith({ kind: 'download', url: 'https://www.youtube.com/watch?v=newVideo002', preset: undefined });
    stop();
  });

  it('the menu offers video, audio and "Choose in NomNom"', async () => {
    const fake = installBrowser();
    setPath('/watch?v=abc123XYZ_-');
    mountPoint(WATCH);
    const stop = mountYoutube();
    await settle();
    (document.querySelector('.nomnom-more') as HTMLButtonElement).click();
    const labels = [...document.querySelectorAll('.nomnom-menu button')].map((b) => b.textContent);
    expect(labels).toEqual(['Video', 'Audio · MP3', 'Audio · M4A', 'Choose in NomNom…']);
    (document.querySelectorAll('.nomnom-menu button')[1] as HTMLButtonElement).click();
    await settle(10);
    expect(fake.sendMessage).toHaveBeenCalledWith({
      kind: 'download',
      url: 'https://www.youtube.com/watch?v=abc123XYZ_-',
      preset: { kind: 'audio', format: 'mp3' },
    });
    stop();
  });
});

describe('Shorts', () => {
  it('puts a compact button on the reel that is playing, and only there', async () => {
    installBrowser();
    setPath('/shorts/abcDEF12345');
    mountPoint(SHORTS);
    const stop = mountYoutube();
    await settle();
    expect(count('[data-nomnom]')).toBe(1);
    const active = document.querySelector('ytd-reel-video-renderer[is-active]')!;
    expect(count('[data-nomnom]', active)).toBe(1);
    expect(document.querySelector('[data-nomnom]')!.hasAttribute('data-compact')).toBe(true);
    stop();
  });
});

describe('navigation events', () => {
  it('re-adds the button after YouTube says the page changed, with no duplicate', async () => {
    installBrowser();
    setPath('/watch?v=abc123XYZ_-');
    mountPoint(WATCH);
    const stop = run(youtube);
    await settle();
    window.dispatchEvent(new Event('yt-navigate-finish'));
    await settle();
    expect(count('[data-nomnom]')).toBe(1);
    stop();
    expect(count('[data-nomnom]')).toBe(0);
  });
});
