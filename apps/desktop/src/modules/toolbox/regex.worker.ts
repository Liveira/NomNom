import { regexRun } from './logic';

/** Runs in its own thread, so a pattern that never finishes can be stopped from outside. */
self.onmessage = (e: MessageEvent<{ pattern: string; flags: string; text: string }>) => {
  const { pattern, flags, text } = e.data;
  self.postMessage(regexRun(pattern, flags, text));
};
