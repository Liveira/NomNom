/** Category ids used to organize NomNom's tools on the Tools page and in the sidebar. */
export type ToolCategoryId = 'files' | 'media' | 'phone' | 'productivity' | 'gaming' | 'system';

export interface ToolCategory {
  id: ToolCategoryId;
  title: string;
  desc: string;
  icon: string;
}

export const TOOL_CATEGORIES: ToolCategory[] = [
  {
    id: 'files',
    title: 'Files & Storage',
    desc: 'Organize, convert, inspect, compare, archive and back up your files.',
    icon: 'folder',
  },
  {
    id: 'media',
    title: 'Media & Capture',
    desc: 'Download videos and music, capture screenshots and record your screen.',
    icon: 'film',
  },
  {
    id: 'phone',
    title: 'Phone & Sharing',
    desc: 'Control your Android phone, use it as a webcam and share files over Wi-Fi.',
    icon: 'smartphone',
  },
  {
    id: 'productivity',
    title: 'Productivity & Text',
    desc: 'Clipboard, translation, notes, snippets, reminders, timers and quick actions.',
    icon: 'pencil',
  },
  {
    id: 'gaming',
    title: 'Focus & Gaming',
    desc: 'Track active app and game hours and tune your PC while playing.',
    icon: 'playtime',
  },
  {
    id: 'system',
    title: 'System & Developer',
    desc: 'Monitor resources, manage startup apps, tidy PATH and run everyday utilities.',
    icon: 'cpu',
  },
];

/** What each tool is, for the Tools page, the first-time setup and the Home page. */
export interface ToolInfo {
  id: string;
  category: ToolCategoryId;
  icon: string;
  title: string;
  desc: string;
  tint: string;
}

export const TOOLS: ToolInfo[] = [
  {
    id: 'downloads',
    category: 'media',
    icon: 'cat',
    title: 'NomNom Downloader',
    desc: 'Grab videos and music with queue, history and format choice.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'activity',
    category: 'gaming',
    icon: 'playtime',
    title: 'Playtime',
    desc: 'See active hours for your daily apps, creative tools and games.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'mobile',
    category: 'phone',
    icon: 'smartphone',
    title: 'Mobile Manager',
    desc: 'Mirror screen, notifications, controls, messages, calls, photos, apps and webcam for your Android phone.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'storage',
    category: 'files',
    icon: 'storage',
    title: 'Disk Analyzer',
    desc: 'Visual treemap, sunburst, duplicates and deep space cleanup.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'filetools',
    category: 'files',
    icon: 'convert',
    title: 'File Tools',
    desc: 'Convert images, audio, video and Office documents locally.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'organizer',
    category: 'files',
    icon: 'organizer',
    title: 'File Organizer',
    desc: 'Preview-first folder sorting with full one-click undo.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'backup',
    category: 'files',
    icon: 'backup',
    title: 'Backup',
    desc: 'Scheduled folder backups with versions, restore and a check that they are intact.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'clipboard',
    category: 'productivity',
    icon: 'clipboard',
    title: 'Clipboard',
    desc: 'Everything you copy, searchable and kept only on this computer. Pin what you reuse.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'text',
    category: 'productivity',
    icon: 'languages',
    title: 'Translator',
    desc: 'Translate any selected text with DeepL: replace it in place, or read it in a small window.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'phone',
    category: 'phone',
    icon: 'smartphone',
    title: 'Phone transfer',
    desc: 'Send files between your phone and this computer over your Wi-Fi, with a QR code and no cloud.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'startup',
    category: 'system',
    icon: 'power',
    title: 'Startup apps',
    desc: 'See what opens with your computer and how much each one slowed it down. Turn off what you do not need.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'actions',
    category: 'productivity',
    icon: 'rocket',
    title: 'My actions',
    desc: 'Programs, folders, links and commands of your own, with shortcuts and a search box (Ctrl+K anywhere).',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'reminders',
    category: 'productivity',
    icon: 'bell',
    title: 'Reminders',
    desc: 'Say "in 20 minutes stretch" or "tomorrow at 9 call mom". A card appears when it is time.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'notes',
    category: 'productivity',
    icon: 'pencil',
    title: 'Notes',
    desc: 'Short notes kept on this computer. A shortcut opens a blank one at once.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'clocks',
    category: 'productivity',
    icon: 'clock',
    title: 'Time tools',
    desc: 'World clocks, a time-zone converter, a stopwatch and a timer that rings even with the window closed.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'toolbox',
    category: 'system',
    icon: 'wrench',
    title: 'Text toolbox',
    desc: 'JSON, Base64, URLs, hashes, UUIDs, regex, timestamps, text differences and colours, all on this computer.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'mini',
    category: 'system',
    icon: 'wand',
    title: 'Mini tools',
    desc: 'Keep a window on top, pick a colour from the screen, paste without formatting and clean tracking off links.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'inbox',
    category: 'productivity',
    icon: 'bell',
    title: 'Notifications',
    desc: 'The notices NomNom showed in the last days, so one that went away unread can be found again.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'rules',
    category: 'files',
    icon: 'wand',
    title: 'Folder rules',
    desc: 'When a file shows up in a folder, move it, rename it or turn it into WebP for you. Nothing is ever deleted, and every move can be undone.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'snippets',
    category: 'productivity',
    icon: 'file-text',
    title: 'Snippets',
    desc: 'Text you type again and again, saved under a name and pasted from the search, with the date or the clipboard filled in.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'shots',
    category: 'media',
    icon: 'image',
    title: 'Screenshots',
    desc: 'Take a picture of part of the screen and mark it up with arrows, boxes, numbers, text and blocked-out parts. Then copy or save it.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'gamemode',
    category: 'gaming',
    icon: 'playtime',
    title: 'Game mode',
    desc: 'While a game is in front: quieter notices, the fastest power plan and the background programs you choose closed. Put back afterwards.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'qr',
    category: 'phone',
    icon: 'layers',
    title: 'QR codes',
    desc: 'Make a code from text or a Wi-Fi network, or read one that is on your screen.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'compare',
    category: 'files',
    icon: 'arrow',
    title: 'Compare folders',
    desc: 'See what is different between two folders and copy what you choose. Nothing is deleted, and a replaced file keeps its old version.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'archives',
    category: 'files',
    icon: 'package',
    title: 'ZIP files',
    desc: 'Pack files and folders into a ZIP, or unpack one safely into a new folder.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'recorder',
    category: 'media',
    icon: 'film',
    title: 'Screen recorder',
    desc: 'Record the screen, or part of it, as a video or a small GIF. No sound.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'paths',
    category: 'system',
    icon: 'wrench',
    title: 'PATH editor',
    desc: 'See the folders Windows looks in for programs, find the repeated and missing ones, and tidy them with a saved copy to go back to.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'monitor',
    category: 'system',
    icon: 'activity',
    title: 'System Monitor',
    desc: 'Live processor, memory, disks, network and the programs using the most.',
    tint: 'var(--glow-mint)',
  },
];

const TOOL_BY_ID = new Map(TOOLS.map((t) => [t.id, t]));

/** Always in the sidebar: the pages that find and set up everything else. */
export const CORE = new Set(['home', 'tools', 'settings']);

/** What a new install starts with. The rest are one click away on the Tools page. */
export const DEFAULT_PICKS = ['downloads', 'activity', 'mobile', 'organizer', 'filetools'];

/**
 * The pages that go in the sidebar. `picked` is the person's choice; `null` means they never chose,
 * so everything shows (how NomNom behaved before there was a choice).
 */
export function sidebarItems<T extends { id: string }>(nav: T[], picked: string[] | null): T[] {
  if (!picked) return nav;
  const chosen = new Set(picked);
  return nav.filter((n) => CORE.has(n.id) || chosen.has(n.id));
}

/** The choice with one tool switched on or off, keeping the order of the tool list. */
export function withTool(picked: string[] | null, id: string, on: boolean): string[] {
  const current = new Set(picked ?? TOOLS.map((t) => t.id));
  if (on) current.add(id);
  else current.delete(id);
  return TOOLS.map((t) => t.id).filter((t) => current.has(t));
}

/** Switches every tool in a category on or off in the sidebar, keeping canonical tool order. */
export function withCategory(picked: string[] | null, category: ToolCategoryId, on: boolean): string[] {
  const current = new Set(picked ?? TOOLS.map((t) => t.id));
  for (const t of TOOLS) {
    if (t.category !== category) continue;
    if (on) current.add(t.id);
    else current.delete(t.id);
  }
  return TOOLS.map((t) => t.id).filter((t) => current.has(t));
}

/** Groups a list of tools by category in display order, omitting empty categories. */
export function toolsByCategory(list: ToolInfo[] = TOOLS): { category: ToolCategory; tools: ToolInfo[] }[] {
  return TOOL_CATEGORIES.map((category) => ({
    category,
    tools: list.filter((t) => t.category === category.id),
  })).filter((g) => g.tools.length > 0);
}

/** Returns the category metadata for a tool id, or null for core pages. */
export function toolCategoryOf(id: string): ToolCategory | null {
  const tool = TOOL_BY_ID.get(id);
  if (!tool) return null;
  return TOOL_CATEGORIES.find((c) => c.id === tool.category) ?? null;
}
