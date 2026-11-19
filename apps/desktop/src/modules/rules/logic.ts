import type { FolderRule, RuleStep } from '../../lib/ipc';

export const STEP_KINDS: { type: RuleStep['type']; label: string; hint: string }[] = [
  { type: 'move', label: 'Move to a folder', hint: 'The file goes there. A file with the same name is never replaced.' },
  { type: 'copy', label: 'Copy to a folder', hint: 'A copy goes there; the original stays.' },
  { type: 'rename', label: 'Rename', hint: 'Only the name changes, not the extension. For example {date}_{name}.' },
  { type: 'convert', label: 'Turn the picture into another format', hint: 'Makes a new picture. The original stays, or goes to the folder you choose.' },
  { type: 'run', label: 'Run one of my actions', hint: 'The action gets the file as {file}.' },
  { type: 'notify', label: 'Tell me', hint: 'One notice for the files that were handled together.' },
];

export function newStep(type: RuleStep['type']): RuleStep {
  switch (type) {
    case 'move':
      return { type, folder: '' };
    case 'copy':
      return { type, folder: '' };
    case 'rename':
      return { type, template: '{date}_{name}' };
    case 'convert':
      return { type, to: 'webp', originals: '' };
    case 'run':
      return { type, action: '' };
    default:
      return { type: 'notify' };
  }
}

export const blankRule = (folder = ''): FolderRule => ({
  id: '',
  name: '',
  enabled: true,
  folder,
  subfolders: false,
  extensions: [],
  name_has: '',
  min_kb: 0,
  max_kb: 0,
  steps: [newStep('move')],
  since: 0,
});

/** The last part of a path, for short descriptions. */
export function leaf(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

/** The folder a path is in. */
export function parentOf(path: string): string {
  return path.replace(/[\\/]+[^\\/]*[\\/]*$/, '');
}

export function describeStep(s: RuleStep, actionName = (id: string) => id): string {
  switch (s.type) {
    case 'move':
      return `move to ${leaf(s.folder)}`;
    case 'copy':
      return `copy to ${leaf(s.folder)}`;
    case 'rename':
      return `rename as ${s.template}`;
    case 'convert':
      return `make it ${s.to.toUpperCase()}`;
    case 'run':
      return `run ${actionName(s.action)}`;
    default:
      return 'tell me';
  }
}

/** "Pictures (png, jpg) in Downloads: make it WEBP, move to Pictures". */
export function describeRule(r: FolderRule, actionName?: (id: string) => string): string {
  const what = r.extensions.length > 0 ? r.extensions.join(', ') : 'any file';
  const extra = [r.name_has.trim() ? `name has "${r.name_has.trim()}"` : '', r.min_kb > 0 ? `over ${r.min_kb} KB` : '', r.max_kb > 0 ? `under ${r.max_kb} KB` : '']
    .filter(Boolean)
    .join(', ');
  return `${what}${extra ? ` (${extra})` : ''} in ${leaf(r.folder)}: ${r.steps.map((s) => describeStep(s, actionName)).join(', then ')}`;
}

export interface Preset {
  name: string;
  rule: (profile: string, sep: string) => FolderRule;
}

/** Starting points, with the person's own folders filled in. */
export function presets(downloads: string): Preset[] {
  const sep = downloads.includes('\\') ? '\\' : '/';
  const home = downloads.replace(/[\\/]+[^\\/]+[\\/]*$/, '');
  const at = (...p: string[]) => [home, ...p].join(sep);
  const base = (over: Partial<FolderRule>): FolderRule => ({ ...blankRule(downloads), ...over });
  return [
    {
      name: 'Pictures from Downloads become WebP',
      rule: () =>
        base({
          name: 'Downloaded pictures to WebP',
          extensions: ['png', 'jpg', 'jpeg'],
          steps: [{ type: 'convert', to: 'webp', originals: at('Pictures', 'Originals') }, { type: 'move', folder: at('Pictures', 'Converted') }, { type: 'notify' }],
        }),
    },
    {
      name: 'PDFs from Downloads go to Documents',
      rule: () => base({ name: 'PDFs to Documents', extensions: ['pdf'], steps: [{ type: 'move', folder: at('Documents', 'PDFs') }, { type: 'notify' }] }),
    },
    {
      name: 'Screenshots get the date in their name',
      rule: () =>
        base({
          name: 'Dated screenshots',
          folder: at('Pictures', 'Screenshots'),
          extensions: ['png'],
          steps: [{ type: 'rename', template: '{date}_{name}' }],
        }),
    },
  ];
}
