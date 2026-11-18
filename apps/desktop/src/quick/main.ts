import { mount } from 'svelte';
import '../lib/design/fonts.css';
import '../lib/design/tokens.css';
import '../lib/design/base.css';
import { i18n } from '../lib/i18n/index.svelte';
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
    i18n.apply(s.general.language);
  })
  .catch(() => {});

mount(Popup, { target: document.getElementById('app')! });
i18n.start('auto');
