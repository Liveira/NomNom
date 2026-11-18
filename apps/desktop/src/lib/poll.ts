/**
 * Runs `fn` every `ms` milliseconds, but only while the window is showing. A window in the tray or
 * minimised has nobody looking at it, so it should cost nothing. When the window comes back,
 * `fn` runs once straight away so the numbers are fresh. Returns a function that stops it.
 */
export function every(ms: number, fn: () => void): () => void {
  const id = setInterval(() => {
    if (!document.hidden) fn();
  }, ms);
  const back = () => {
    if (!document.hidden) fn();
  };
  document.addEventListener('visibilitychange', back);
  return () => {
    clearInterval(id);
    document.removeEventListener('visibilitychange', back);
  };
}
