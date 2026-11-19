import type { ActionKind, ActionParam, CustomAction } from '../../lib/ipc';

export const KINDS: { id: ActionKind; label: string; icon: string; target: string; hint: string }[] = [
  { id: 'app', label: 'Program', icon: 'rocket', target: 'Program to start', hint: 'Starts a program and lets it go.' },
  { id: 'folder', label: 'Folder or file', icon: 'folder', target: 'Folder or file to open', hint: 'Opens it the way a double click would.' },
  { id: 'link', label: 'Web link', icon: 'link', target: 'Web address', hint: 'Opens the address in your browser.' },
  { id: 'command', label: 'Command', icon: 'terminal', target: 'Program to run', hint: 'Runs a program and shows what it prints.' },
];

export const blankAction = (): CustomAction => ({
  id: '',
  name: '',
  kind: 'app',
  target: '',
  args: '',
  params: [],
  shortcut: null,
  in_palette: true,
  cwd: null,
});

export const blankParam = (name = ''): ActionParam => ({ name, label: name, default: '', kind: 'text' });

/** The `{name}` values written in a text, once each, in order. */
export function placeholders(text: string): string[] {
  const seen: string[] = [];
  for (const m of text.matchAll(/\{([A-Za-z_][A-Za-z0-9_]{0,31})\}/g)) {
    if (!seen.includes(m[1])) seen.push(m[1]);
  }
  return seen;
}

/** Values used in the target or arguments that the action doesn't list yet. */
export function missingValues(a: Pick<CustomAction, 'target' | 'args' | 'params'>): string[] {
  const known = new Set(a.params.map((p) => p.name));
  return [...placeholders(a.target), ...placeholders(a.args)].filter((n, i, all) => !known.has(n) && all.indexOf(n) === i);
}

/** A short line for the list: what it opens or runs. */
export function summary(a: Pick<CustomAction, 'kind' | 'target' | 'args'>): string {
  return a.kind === 'app' || a.kind === 'command' ? `${a.target}${a.args.trim() ? ' ' + a.args.trim() : ''}` : a.target;
}

/** Texts to try from the empty form, so nobody starts from a blank page. */
export const EXAMPLES: { name: string; kind: ActionKind; target: string; args: string; params: ActionParam[] }[] = [
  { name: 'Search the web', kind: 'link', target: 'https://duckduckgo.com/?q={q}', args: '', params: [{ name: 'q', label: 'What to search for', default: '', kind: 'text' }] },
  { name: 'Open my projects folder', kind: 'folder', target: 'C:\Users\you\Projects', args: '', params: [] },
];
