import type { Entry } from '../engine';
import { FEATURES } from './features';
import { FILETOOLS } from './filetools';
import { MESSAGES } from './messages';
import { MISC } from './misc';
import { PLAYTIME } from './playtime';
import { SETTINGS } from './settings';
import { SHELL } from './shell';
import { STORAGE } from './storage';
import { STORAGE2 } from './storage2';

/** Every translated string, [English, Português, Español]. Add new text to the file of its area. */
export const CATALOG: Entry[] = [...SHELL, ...SETTINGS, ...PLAYTIME, ...FILETOOLS, ...STORAGE, ...STORAGE2, ...MISC, ...MESSAGES, ...FEATURES];
