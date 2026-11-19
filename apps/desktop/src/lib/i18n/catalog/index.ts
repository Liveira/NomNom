import type { Entry } from '../engine';
import { ACTIONS } from './actions';
import { BACKUP } from './backup';
import { EVERYDAY, MINI } from './everyday';
import { FEATURES } from './features';
import { FILETOOLS } from './filetools';
import { MESSAGES } from './messages';
import { MISC } from './misc';
import { MONITOR } from './monitor';
import { PLAYTIME } from './playtime';
import { REMINDERS } from './reminders';
import { SETTINGS } from './settings';
import { SHELL } from './shell';
import { STORAGE } from './storage';
import { STORAGE2 } from './storage2';
import { TOOLS } from './tools';

/** Every translated string, [English, Português, Español]. Add new text to the file of its area. */
export const CATALOG: Entry[] = [...SHELL, ...SETTINGS, ...PLAYTIME, ...FILETOOLS, ...STORAGE, ...STORAGE2, ...MISC, ...MESSAGES, ...FEATURES, ...BACKUP, ...MONITOR, ...TOOLS, ...ACTIONS, ...REMINDERS, ...EVERYDAY, ...MINI];
