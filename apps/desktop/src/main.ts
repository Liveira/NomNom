import { mount } from 'svelte';
import './lib/design/fonts.css';
import './lib/design/tokens.css';
import './lib/design/base.css';
import Shell from './app/Shell.svelte';
import { app } from './app/state.svelte';
import { ipc, onBrowserOpen, onStorageInbox, onTasks, stIpc } from './lib/ipc';
import { downloads } from './modules/downloads/store.svelte';

async function start() {
  await app.init();
  mount(Shell, { target: document.getElementById('app')! });
  await onTasks((batch) => app.mergeTasks(batch));
  void downloads.start();
  await onBrowserOpen((url) => {
    downloads.pendingUrl = url;
    app.go('downloads');
  });
  await onStorageInbox(async () => {
    app.go('storage');
    const mod = await import('./modules/storage/store.svelte');
    await mod.storage.checkInbox();
  });
  const stFolders = await stIpc.takeInbox().catch(() => [] as string[]);
  if (stFolders.length > 0) {
    app.go('storage');
    const mod = await import('./modules/storage/store.svelte');
    for (const folder of stFolders) await mod.storage.start(folder);
  }
  // Report time-to-first-render so startup can be measured on release builds.
  requestAnimationFrame(() => void ipc.appReady(performance.now()));
}

start().catch((e) => {
  document.getElementById('app')!.textContent = `NomNom couldn't start: ${e}`;
});
