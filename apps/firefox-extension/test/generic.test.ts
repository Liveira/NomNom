import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { bigVideoAt, mountPlayerButton, videoPageUrl } from '../src/adapters/generic';
import { ALL_SITES, PLAYER_SITES } from '../src/shared/sites';
import { installBrowser, mountPoint, settle } from './helpers';

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

/** jsdom has no layout, so a video is given a size by hand. */
function sizeIt(el: Element, w = 640, h = 360, left = 100, top = 100) {
  el.getBoundingClientRect = () => ({ width: w, height: h, left, top, right: left + w, bottom: top + h, x: left, y: top, toJSON() {} }) as DOMRect;
}

describe('the player button', () => {
  it('finds a big video under the pointer and ignores small ones', () => {
    mountPoint('<video id="big"></video><video id="tiny"></video>');
    sizeIt(document.getElementById('big')!);
    sizeIt(document.getElementById('tiny')!, 60, 40, 0, 0);
    expect(bigVideoAt(document, 300, 200)?.id).toBe('big');
    expect(bigVideoAt(document, 10, 10)).toBeNull();
    expect(bigVideoAt(document, 900, 900)).toBeNull();
  });

  it('shows over the video when the pointer is on it, and goes away after it leaves', async () => {
    mountPoint('<video id="v"></video>');
    sizeIt(document.getElementById('v')!);
    const stop = mountPlayerButton(document, { href: 'https://vimeo.com/123456' });
    const holder = document.querySelector('.nomnom-player') as HTMLElement;
    expect(holder.style.display).toBe('none');
    document.dispatchEvent(new MouseEvent('mousemove', { clientX: 300, clientY: 200, bubbles: true }));
    expect(holder.style.display).toBe('block');
    await settle(100);
    document.dispatchEvent(new MouseEvent('mousemove', { clientX: 900, clientY: 900, bubbles: true }));
    await settle(1000);
    expect(holder.style.display).toBe('none');
    stop();
    expect(document.querySelector('.nomnom-player')).toBeNull();
  });

  it('sends the address of the page for a video of its own, from the options menu', async () => {
    const fake = installBrowser();
    mountPoint('<video id="v"></video>');
    sizeIt(document.getElementById('v')!);
    const stop = mountPlayerButton(document, { href: 'https://vimeo.com/123456#t=3' });
    document.dispatchEvent(new MouseEvent('mousemove', { clientX: 300, clientY: 200, bubbles: true }));
    (document.querySelector('.nomnom-player .nomnom-main') as HTMLButtonElement).click();
    const labels = [...document.querySelectorAll('.nomnom-menu button')].map((b) => b.textContent);
    expect(labels).toEqual(['Video', 'Audio · MP3', 'Audio · M4A', 'Choose in NomNom…']);
    (document.querySelectorAll('.nomnom-menu button')[1] as HTMLButtonElement).click();
    await settle(10);
    expect(fake.sendMessage).toHaveBeenCalledWith({ kind: 'download', url: 'https://vimeo.com/123456', preset: { kind: 'audio', format: 'mp3' } });
    stop();
  });
});

describe('which link belongs to a video', () => {
  it('prefers the link around the player in a feed', () => {
    mountPoint('<div><a href="/@someone/video/7001"><div><video id="v"></video></div></a></div>');
    expect(videoPageUrl(document.getElementById('v')!, { href: 'https://www.tiktok.com/foryou' })).toBe('https://www.tiktok.com/@someone/video/7001');
  });

  it('falls back to the address bar, without the #fragment', () => {
    mountPoint('<video id="v"></video>');
    expect(videoPageUrl(document.getElementById('v')!, { href: 'https://vimeo.com/55#t=9' })).toBe('https://vimeo.com/55');
  });

  it('never returns a link from another site', () => {
    mountPoint('<a href="https://evil.example/video/1"><video id="v"></video></a>');
    expect(videoPageUrl(document.getElementById('v')!, { href: 'https://example.org/page' })).toBeNull();
  });
});

describe('what ships', () => {
  const manifest = JSON.parse(readFileSync('manifest.json', 'utf8'));

  it('the manifest covers exactly the sites the extension knows', () => {
    const hosts = (manifest.host_permissions as string[]).map((h) => h.replace('https://', '').replace(/^(www|\*)\./, '').replace('/*', ''));
    expect([...new Set(hosts)].sort()).toEqual([...ALL_SITES].sort());
    const generic = manifest.content_scripts.find((c: { js: string[] }) => c.js.includes('content-generic.js'));
    expect(generic.matches.map((m: string) => m.replace('https://*.', '').replace('/*', '')).sort()).toEqual([...PLAYER_SITES].sort());
  });

  it("the Chrome extension's ID matches the one the NomNom app allows", () => {
    const der = Buffer.from(readFileSync('chrome/public-key.txt', 'utf8').trim(), 'base64');
    const id = [...createHash('sha256').update(der).digest().subarray(0, 16)].map((b) => String.fromCharCode(97 + (b >> 4)) + String.fromCharCode(97 + (b & 15))).join('');
    expect(id).toBe('kejgmoigmjpbghoihdjlahegijloeghc');
  });
});
