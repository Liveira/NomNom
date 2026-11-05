import { mount } from 'svelte';
import './lib/design/tokens.css';
import './lib/design/base.css';
import Shell from './app/Shell.svelte';
import { app } from './app/state.svelte';
import { ipc, onTasks } from './lib/ipc';
import { downloads } from './modules/downloads/store.svelte';

async function start() {
  await app.init();
  mount(Shell, { target: document.getElementById('app')! });
  await onTasks((batch) => app.mergeTasks(batch));
  void downloads.start();
  // Report time-to-first-render so startup can be measured on release builds.
  requestAnimationFrame(() => void ipc.appReady(performance.now()));
}

start().catch((e) => {
  document.getElementById('app')!.textContent = `NOMNOM couldn't start: ${e}`;
});
