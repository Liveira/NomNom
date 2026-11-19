import type { Match, Result } from './logic';

/** Tries a pattern in a worker and gives up after `ms`, so a runaway pattern cannot freeze the window. */
export function runRegex(pattern: string, flags: string, text: string, ms = 1500): Promise<Result<Match[]>> {
  return new Promise((resolve) => {
    const worker = new Worker(new URL('./regex.worker.ts', import.meta.url), { type: 'module' });
    const timer = setTimeout(() => {
      worker.terminate();
      resolve({ ok: false, error: 'That pattern takes too long on this text, so it was stopped. Try a simpler one.' });
    }, ms);
    worker.onmessage = (e: MessageEvent<Result<Match[]>>) => {
      clearTimeout(timer);
      worker.terminate();
      resolve(e.data);
    };
    worker.onerror = () => {
      clearTimeout(timer);
      worker.terminate();
      resolve({ ok: false, error: 'The pattern could not be tested.' });
    };
    worker.postMessage({ pattern, flags, text });
  });
}
