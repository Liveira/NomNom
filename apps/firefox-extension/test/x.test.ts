import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { hasOwnMedia, hasOwnVideo, mountX, photoUrls, tweetUrl } from '../src/adapters/x';
import { count, installBrowser, mountPoint, settle } from './helpers';

const photo = (id: string) =>
  `<div data-testid="tweetPhoto"><img src="https://pbs.twimg.com/media/${id}?format=jpg&name=small" alt="Image"></div>`;
const row = '<div role="group"><div data-testid="reply"></div><div data-testid="retweet"></div><div data-testid="like"></div></div>';
const when = (user: string, id: string) => `<div><a href="/${user}/status/${id}"><time datetime="2026-01-01"></time></a></div>`;

function tweet(user: string, id: string, media = '', quoted = ''): string {
  return `<article data-testid="tweet">${when(user, id)}<div data-testid="tweetText">text</div>${media}${quoted}${row}</article>`;
}
const quote = (user: string, id: string, media: string) =>
  `<div role="link" tabindex="0"><div data-testid="tweetText">quoted</div>${when(user, id)}${media}</div>`;
const video = '<div data-testid="videoPlayer"><video></video></div>';

beforeEach(() => vi.useFakeTimers());
afterEach(() => {
  vi.useRealTimers();
  document.body.innerHTML = '';
});

describe('reading a post', () => {
  it('uses the post\'s own timestamp for its link', () => {
    mountPoint(tweet('alice', '111', photo('AAA')));
    expect(tweetUrl(document.querySelector('article')!)).toBe('https://x.com/alice/status/111');
  });

  it('does not take the link of the post it quotes', () => {
    mountPoint(tweet('alice', '111', photo('AAA'), quote('bob', '222', photo('QQQ'))));
    expect(tweetUrl(document.querySelector('article')!)).toBe('https://x.com/alice/status/111');
  });

  it('a post that only quotes something with media has no media of its own', () => {
    mountPoint(tweet('alice', '111', '', quote('bob', '222', photo('QQQ') + video)));
    const a = document.querySelector('article')!;
    expect(hasOwnMedia(a)).toBe(false);
    expect(hasOwnVideo(a)).toBe(false);
    expect(photoUrls(a)).toEqual([]);
  });

  it('collects every one of its own photos at full size, in order, ignoring the quoted post\'s', () => {
    mountPoint(tweet('alice', '111', photo('AAA') + photo('BBB') + photo('CCC'), quote('bob', '222', photo('QQQ'))));
    expect(photoUrls(document.querySelector('article')!)).toEqual([
      'https://pbs.twimg.com/media/AAA?format=jpg&name=orig',
      'https://pbs.twimg.com/media/BBB?format=jpg&name=orig',
      'https://pbs.twimg.com/media/CCC?format=jpg&name=orig',
    ]);
  });

  it('ignores images that are not post photos (avatars, links to other hosts)', () => {
    mountPoint(
      tweet(
        'alice',
        '111',
        '<div data-testid="tweetPhoto"><img src="https://evil.example/media/x.jpg"></div><div data-testid="tweetPhoto"><img src="https://pbs.twimg.com/profile_images/1/a.jpg"></div>',
      ),
    );
    expect(photoUrls(document.querySelector('article')!)).toEqual([]);
  });
});

describe('the buttons', () => {
  it('appear on posts with media, next to Reply and Like, and not on text posts', async () => {
    installBrowser();
    mountPoint(
      tweet('alice', '1', photo('AAA')) + tweet('bob', '2') + tweet('carol', '3', video) + tweet('dave', '4', '', quote('erin', '5', photo('QQQ'))),
    );
    const stop = mountX();
    await settle();
    const articles = [...document.querySelectorAll('article')];
    expect(articles.map((a) => count('[data-nomnom]', a))).toEqual([1, 0, 1, 0]);
    expect(articles[0].querySelector('[role="group"]')!.lastElementChild!.hasAttribute('data-nomnom')).toBe(true);
    stop();
  });

  it('keep up with infinite scrolling and never double up', async () => {
    installBrowser();
    const body = mountPoint(tweet('a', '1', photo('A1')));
    const stop = mountX();
    await settle();
    for (let i = 2; i <= 30; i++) {
      body.insertAdjacentHTML('beforeend', tweet('user', String(i), i % 2 ? photo('P' + i) : ''));
      await settle(40);
    }
    await settle();
    const withMedia = [...document.querySelectorAll('article')].filter((a) => hasOwnMedia(a));
    // the first post plus every odd-numbered one from 3 to 29
    expect(withMedia.length).toBe(15);
    for (const a of withMedia) expect(count('[data-nomnom]', a)).toBe(1);
    stop();
    expect(count('[data-nomnom]')).toBe(0);
  });

  it('a photo post sends its own photos and its own link', async () => {
    const fake = installBrowser();
    mountPoint(tweet('alice', '111', photo('AAA') + photo('BBB'), quote('bob', '222', photo('QQQ'))));
    const stop = mountX();
    await settle();
    (document.querySelector('.nomnom-main') as HTMLButtonElement).click();
    await settle(10);
    expect(fake.sendMessage).toHaveBeenCalledWith({
      kind: 'media',
      source: 'https://x.com/alice/status/111',
      media: ['https://pbs.twimg.com/media/AAA?format=jpg&name=orig', 'https://pbs.twimg.com/media/BBB?format=jpg&name=orig'],
    });
    stop();
  });

  it('a video or GIF post sends the post link for the video downloader', async () => {
    const fake = installBrowser();
    mountPoint(tweet('carol', '333', video));
    const stop = mountX();
    await settle();
    (document.querySelector('.nomnom-main') as HTMLButtonElement).click();
    await settle(10);
    expect(fake.sendMessage).toHaveBeenCalledWith({ kind: 'download', url: 'https://x.com/carol/status/333', preset: undefined });
    stop();
  });

  it('on a timeline, each button downloads its own post', async () => {
    const fake = installBrowser();
    mountPoint(tweet('a', '1', photo('ONE')) + tweet('b', '2', photo('TWO')) + tweet('c', '3', photo('THREE')));
    const stop = mountX();
    await settle();
    const buttons = [...document.querySelectorAll<HTMLButtonElement>('.nomnom-main')];
    buttons[1].click();
    await settle(10);
    expect(fake.sendMessage).toHaveBeenCalledTimes(1);
    expect(fake.sendMessage.mock.calls[0][0]).toMatchObject({ source: 'https://x.com/b/status/2', media: ['https://pbs.twimg.com/media/TWO?format=jpg&name=orig'] });
    stop();
  });
});
