import { vi } from 'vitest';

export interface FakeBrowser {
  sendMessage: ReturnType<typeof vi.fn>;
  sendNativeMessage: ReturnType<typeof vi.fn>;
}

/** A stand-in for Firefox's `browser` object. */
export function installBrowser(reply: unknown = { ok: true, message: 'Added to NomNom.' }): FakeBrowser {
  const fake: FakeBrowser = {
    sendMessage: vi.fn().mockResolvedValue(reply),
    sendNativeMessage: vi.fn(),
  };
  (globalThis as unknown as { browser: unknown }).browser = {
    runtime: {
      id: 'nomnom@nomnom.local',
      sendMessage: fake.sendMessage,
      sendNativeMessage: fake.sendNativeMessage,
      getURL: (p: string) => `moz-extension://abc/${p}`,
      onMessage: { addListener: vi.fn() },
    },
  };
  return fake;
}

/** Runs the pending timers and microtasks of a burst of DOM changes. */
export async function settle(ms = 400): Promise<void> {
  await vi.advanceTimersByTimeAsync(ms);
}

export function setPath(path: string): void {
  history.pushState({}, '', path);
}

export function mountPoint(html: string): HTMLElement {
  document.head.innerHTML = '';
  document.body.innerHTML = html;
  return document.body;
}

export const count = (sel: string, root: ParentNode = document) => root.querySelectorAll(sel).length;
