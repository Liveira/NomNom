import { mount } from 'svelte';
import '../lib/design/tokens.css';
import '../lib/design/base.css';
import { ipc } from '../lib/ipc';
import Popup from './Popup.svelte';

// Follow the saved theme without loading the rest of the app.
const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
document.documentElement.dataset.theme = dark ? 'dark' : 'light';
ipc
  .getSettings()
  .then((s) => {
    const t = s.appearance.theme;
    document.documentElement.dataset.theme = t === 'system' ? (dark ? 'dark' : 'light') : t;
  })
  .catch(() => {});

mount(Popup, { target: document.getElementById('app')! });
