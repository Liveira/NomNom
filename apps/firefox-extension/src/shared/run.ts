export interface Adapter {
  /** Adds buttons and returns a function that undoes it. */
  mount(): () => void;
  /** Window events after which the page has navigated and `mount` should run again. */
  navEvents?: string[];
}

/** Mounts the adapter, re-mounts on in-page navigation, and cleans up when the page goes away. */
export function run(adapter: Adapter, win: Window = window): () => void {
  let undo = adapter.mount();
  const remount = () => {
    undo();
    undo = adapter.mount();
  };
  for (const name of adapter.navEvents ?? []) win.addEventListener(name, remount);
  const stop = () => {
    undo();
    for (const name of adapter.navEvents ?? []) win.removeEventListener(name, remount);
  };
  win.addEventListener('pagehide', stop, { once: true });
  return stop;
}
