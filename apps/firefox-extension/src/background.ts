/** The only part of the extension that talks to the NomNom app. Page scripts ask it to send a link;
 *  it checks the request, passes it to the native helper, and turns the answer into one sentence. */
import type { Result, ToBackground } from './shared/messaging';

export const HOST = 'com.nomnom.host';
const SITES = ['youtube.com', 'x.com', 'twitter.com', 'instagram.com', 'reddit.com'];

export function allowedHost(url: unknown): boolean {
  if (typeof url !== 'string') return false;
  try {
    const u = new URL(url);
    return u.protocol === 'https:' && SITES.some((s) => u.hostname === s || u.hostname.endsWith('.' + s));
  } catch {
    return false;
  }
}

function isHttps(url: unknown): boolean {
  try {
    return typeof url === 'string' && new URL(url).protocol === 'https:';
  } catch {
    return false;
  }
}

/** Accepts only the exact message shapes the page scripts send. */
export function validate(raw: unknown): ToBackground | null {
  if (typeof raw !== 'object' || raw === null) return null;
  const m = raw as Record<string, unknown>;
  switch (m.kind) {
    case 'ping':
      return { kind: 'ping' };
    case 'open':
      return allowedHost(m.url) ? { kind: 'open', url: m.url as string } : null;
    case 'download': {
      if (!allowedHost(m.url)) return null;
      const p = m.preset as Record<string, unknown> | undefined;
      if (p !== undefined && (typeof p !== 'object' || (p.kind !== 'video' && p.kind !== 'audio'))) return null;
      return {
        kind: 'download',
        url: m.url as string,
        preset: p
          ? {
              kind: p.kind as 'video' | 'audio',
              ...(typeof p.format === 'string' ? { format: p.format } : {}),
              ...(typeof p.height === 'number' ? { height: p.height } : {}),
            }
          : undefined,
      };
    }
    case 'media': {
      const media = m.media;
      if (!allowedHost(m.source) || !Array.isArray(media) || media.length === 0 || media.length > 20) return null;
      if (!media.every(isHttps)) return null;
      return { kind: 'media', source: m.source as string, media: media as string[] };
    }
    default:
      return null;
  }
}

interface NativeAnswer {
  ok?: boolean;
  duplicate?: boolean;
  app?: string;
  tasks?: string[];
  error?: { code?: string; message?: string };
}

export const NOT_SET_UP =
  "NomNom isn't set up for Firefox yet. Open NomNom, go to Settings, then Browser, and press Connect Firefox.";

export function describe(answer: NativeAnswer | undefined): Result {
  if (!answer || typeof answer !== 'object') return { ok: false, message: "NomNom didn't answer." };
  if (answer.ok) {
    return answer.duplicate
      ? { ok: true, duplicate: true, message: 'Already in NomNom.' }
      : { ok: true, message: 'Added to NomNom.', version: answer.app };
  }
  switch (answer.error?.code) {
    case 'not_running':
      return { ok: false, message: "NomNom isn't running. Open it, or let Firefox start it in Settings, then Browser." };
    case 'unsupported':
      return { ok: false, message: "NomNom can't download from this site." };
    case 'rate_limited':
      return { ok: false, message: 'Too many requests. Wait a moment.' };
    case 'invalid':
      return { ok: false, message: answer.error.message || "NomNom couldn't use this link." };
    default:
      return { ok: false, message: 'NomNom had a problem. Try again.' };
  }
}

type NativeSender = (host: string, message: object) => Promise<unknown>;

export async function handle(msg: ToBackground, sendNative: NativeSender = (h, m) => browser.runtime.sendNativeMessage(h, m)): Promise<Result> {
  const id = crypto.randomUUID();
  let body: object;
  switch (msg.kind) {
    case 'ping':
      body = { v: 1, id, type: 'ping' };
      break;
    case 'open':
      body = { v: 1, id, type: 'open_in_app', url: msg.url };
      break;
    case 'download':
      body = { v: 1, id, type: 'enqueue', url: msg.url, ...(msg.preset ? { preset: msg.preset } : {}) };
      break;
    case 'media':
      body = { v: 1, id, type: 'enqueue_direct', source: msg.source, media: msg.media };
      break;
  }
  try {
    const result = describe((await sendNative(HOST, body)) as NativeAnswer);
    if (msg.kind === 'ping' && result.ok) result.message = 'NomNom is connected.';
    return result;
  } catch (e) {
    const text = String((e as Error)?.message ?? e);
    return /native application|not found|no such/i.test(text)
      ? { ok: false, message: NOT_SET_UP }
      : { ok: false, message: "Couldn't reach NomNom." };
  }
}

function senderAllowed(sender: { id?: string; tab?: { url?: string }; url?: string }, ownRoot: string): boolean {
  if (sender.id !== browser.runtime.id) return false;
  return allowedHost(sender.tab?.url) || (typeof sender.url === 'string' && sender.url.startsWith(ownRoot));
}

export function init(): void {
  const ownRoot = browser.runtime.getURL('');
  browser.runtime.onMessage.addListener((raw: unknown, sender) => {
    const msg = validate(raw);
    if (!msg || !senderAllowed(sender, ownRoot)) {
      return Promise.resolve<Result>({ ok: false, message: 'NomNom ignored an unexpected request.' });
    }
    return handle(msg);
  });
}

if (typeof browser !== 'undefined' && browser.runtime?.onMessage) init();
