/** Runs `fn` now and again whenever `root` changes, but at most once per `wait` ms, so a busy
 *  page costs one cheap query per burst instead of one per mutation. Returns a stop function. */
export function watch(root: Node, fn: () => void, wait = 150): () => void {
  let timer: ReturnType<typeof setTimeout> | undefined;
  const observer = new MutationObserver(() => {
    if (timer) return;
    timer = setTimeout(() => {
      timer = undefined;
      fn();
    }, wait);
  });
  observer.observe(root, { childList: true, subtree: true });
  fn();
  return () => {
    observer.disconnect();
    if (timer) clearTimeout(timer);
  };
}
