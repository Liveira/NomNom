import { beforeEach, describe, expect, it, vi } from 'vitest';
import { allowedHost, describe as describeAnswer, handle, HOST, NOT_SET_UP, validate } from '../src/background';
import { installBrowser } from './helpers';

beforeEach(() => installBrowser());

describe('which pages may be sent', () => {
  it('accepts https links on the supported sites and their subdomains', () => {
    for (const ok of ['https://www.youtube.com/watch?v=a', 'https://x.com/a/status/1', 'https://twitter.com/a/status/1', 'https://www.instagram.com/p/x/', 'https://old.reddit.com/r/a', 'https://m.youtube.com/watch?v=a']) {
      expect(allowedHost(ok), ok).toBe(true);
    }
  });
  it('refuses everything else, including look-alikes', () => {
    for (const bad of ['http://www.youtube.com/watch?v=a', 'https://evilyoutube.com/', 'https://youtube.com.evil.example/', 'https://example.com/', 'file:///C:/x', 'javascript:alert(1)', 'nonsense', '', 5, null, undefined]) {
      expect(allowedHost(bad), String(bad)).toBe(false);
    }
  });
});

describe('validating what a page script sends', () => {
  it('passes well-formed requests through unchanged', () => {
    expect(validate({ kind: 'ping' })).toEqual({ kind: 'ping' });
    expect(validate({ kind: 'open', url: 'https://x.com/a/status/1' })).toEqual({ kind: 'open', url: 'https://x.com/a/status/1' });
    expect(validate({ kind: 'download', url: 'https://www.youtube.com/watch?v=a', preset: { kind: 'audio', format: 'mp3' } })).toEqual({
      kind: 'download',
      url: 'https://www.youtube.com/watch?v=a',
      preset: { kind: 'audio', format: 'mp3' },
    });
    expect(validate({ kind: 'media', source: 'https://x.com/a/status/1', media: ['https://pbs.twimg.com/media/A'] })).toMatchObject({ kind: 'media' });
  });

  it('drops extra fields instead of forwarding them', () => {
    const v = validate({ kind: 'download', url: 'https://x.com/a/status/1', exec: 'calc', preset: { kind: 'video', evil: 1 } }) as { preset: object };
    expect(JSON.stringify(v)).not.toContain('exec');
    expect(JSON.stringify(v)).not.toContain('evil');
  });

  it('refuses anything malformed', () => {
    for (const bad of [
      null,
      'ping',
      {},
      { kind: 'format_disk' },
      { kind: 'download', url: 'https://example.com/v' },
      { kind: 'download', url: 'https://x.com/a/status/1', preset: { kind: 'gif' } },
      { kind: 'download', url: 'https://x.com/a/status/1', preset: 'audio' },
      { kind: 'media', source: 'https://x.com/a', media: [] },
      { kind: 'media', source: 'https://x.com/a', media: ['http://pbs.twimg.com/a.jpg'] },
      { kind: 'media', source: 'https://x.com/a', media: Array(21).fill('https://pbs.twimg.com/a.jpg') },
      { kind: 'media', source: 'https://example.com', media: ['https://pbs.twimg.com/a.jpg'] },
      { kind: 'open', url: 'https://example.com' },
    ]) {
      expect(validate(bad), JSON.stringify(bad)).toBeNull();
    }
  });
});

describe('sending to NomNom', () => {
  it('asks the native helper for a download with the right message', async () => {
    const fake = installBrowser();
    fake.sendNativeMessage.mockResolvedValue({ v: 1, id: 'x', ok: true, tasks: ['t1'], duplicate: false });
    const r = await handle({ kind: 'download', url: 'https://www.youtube.com/watch?v=a' });
    expect(r).toMatchObject({ ok: true, message: 'Added to NomNom.' });
    const [host, body] = fake.sendNativeMessage.mock.calls[0];
    expect(host).toBe(HOST);
    expect(body).toMatchObject({ v: 1, type: 'enqueue', url: 'https://www.youtube.com/watch?v=a' });
    expect(body).not.toHaveProperty('preset');
    expect(typeof body.id).toBe('string');
  });

  it('passes a chosen preset along', async () => {
    const fake = installBrowser();
    fake.sendNativeMessage.mockResolvedValue({ ok: true });
    await handle({ kind: 'download', url: 'https://www.youtube.com/watch?v=a', preset: { kind: 'audio', format: 'mp3' } });
    expect(fake.sendNativeMessage.mock.calls[0][1]).toMatchObject({ type: 'enqueue', preset: { kind: 'audio', format: 'mp3' } });
  });

  it('sends pictures as a direct download from the post', async () => {
    const fake = installBrowser();
    fake.sendNativeMessage.mockResolvedValue({ ok: true });
    await handle({ kind: 'media', source: 'https://x.com/a/status/1', media: ['https://pbs.twimg.com/media/A'] });
    expect(fake.sendNativeMessage.mock.calls[0][1]).toMatchObject({ type: 'enqueue_direct', source: 'https://x.com/a/status/1', media: ['https://pbs.twimg.com/media/A'] });
  });

  it('can open a link in the app and can check the connection', async () => {
    const fake = installBrowser();
    fake.sendNativeMessage.mockResolvedValue({ ok: true, app: '0.1.0' });
    expect(await handle({ kind: 'open', url: 'https://x.com/a/status/1' })).toMatchObject({ ok: true });
    expect(fake.sendNativeMessage.mock.calls[0][1]).toMatchObject({ type: 'open_in_app' });
    expect(await handle({ kind: 'ping' })).toMatchObject({ ok: true, message: 'NomNom is connected.', version: '0.1.0' });
  });

  it('says so when the native helper has not been set up, without NomNom running at all', async () => {
    const fake = installBrowser();
    fake.sendNativeMessage.mockRejectedValue(new Error('No such native application com.nomnom.host'));
    expect(await handle({ kind: 'ping' })).toEqual({ ok: false, message: NOT_SET_UP });
    expect(NOT_SET_UP).toContain('Connect Firefox');
  });

  it('turns other failures into a plain sentence', async () => {
    const fake = installBrowser();
    fake.sendNativeMessage.mockRejectedValue(new Error('pipe broke'));
    expect(await handle({ kind: 'ping' })).toEqual({ ok: false, message: "Couldn't reach NomNom." });
  });
});

describe('what each answer means to the user', () => {
  it('maps every outcome to one sentence', () => {
    expect(describeAnswer({ ok: true })).toMatchObject({ ok: true, message: 'Added to NomNom.' });
    expect(describeAnswer({ ok: true, duplicate: true })).toMatchObject({ ok: true, duplicate: true, message: 'Already in NomNom.' });
    expect(describeAnswer({ ok: false, error: { code: 'not_running' } }).message).toMatch(/isn't running/);
    expect(describeAnswer({ ok: false, error: { code: 'unsupported' } }).message).toMatch(/can't download from this site/);
    expect(describeAnswer({ ok: false, error: { code: 'rate_limited' } }).message).toMatch(/Too many/);
    expect(describeAnswer({ ok: false, error: { code: 'invalid', message: 'That format isn\'t available.' } }).message).toBe("That format isn't available.");
    expect(describeAnswer({ ok: false, error: { code: 'internal' } }).message).toMatch(/problem/);
    expect(describeAnswer(undefined)).toMatchObject({ ok: false });
  });
});
