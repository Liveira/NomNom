import { mount, type Component } from 'svelte';
import '../lib/design/fonts.css';
import '../lib/design/tokens.css';
import '../lib/design/base.css';
import { i18n } from '../lib/i18n/index.svelte';
import { ipc } from '../lib/ipc';
import { isLinux } from '../lib/platform';
import ClipboardPanel from './ClipboardPanel.svelte';
import LauncherPanel from './LauncherPanel.svelte';
import OcrPanel from './OcrPanel.svelte';
import Toast from './Toast.svelte';
import TranslatePanel from './TranslatePanel.svelte';

/** One small page for NomNom's little windows. `?kind=` says which one this is. */
const kind = new URLSearchParams(location.search).get('kind') ?? 'toast';

const dark = window.matchMedia('(prefers-color-scheme: dark)').matches;
document.documentElement.dataset.theme = dark ? 'dark' : 'light';
if (isLinux) document.documentElement.dataset.opaque = 'true';
ipc
  .getSettings()
  .then((s) => {
    const t = s.appearance.theme;
    document.documentElement.dataset.theme = t === 'system' ? (dark ? 'dark' : 'light') : t;
    i18n.apply(s.general.language);
  })
  .catch(() => {});

const views: Record<string, Component> = { toast: Toast, clipboard: ClipboardPanel, translate: TranslatePanel, ocr: OcrPanel, launcher: LauncherPanel };
mount(views[kind] ?? Toast, { target: document.getElementById('app')! });
i18n.start('auto');
