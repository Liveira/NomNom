/** What each tool is, for the Tools page, the first-time setup and the Home page. */
export interface ToolInfo {
  id: string;
  icon: string;
  title: string;
  desc: string;
  tint: string;
}

export const TOOLS: ToolInfo[] = [
  {
    id: 'downloads',
    icon: 'cat',
    title: 'NomNom Downloader',
    desc: 'Grab videos and music with queue, history and format choice.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'activity',
    icon: 'playtime',
    title: 'Playtime',
    desc: 'See active hours for your daily apps, creative tools and games.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'storage',
    icon: 'storage',
    title: 'Disk Analyzer',
    desc: 'Visual treemap, sunburst, duplicates and deep space cleanup.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'filetools',
    icon: 'convert',
    title: 'File Tools',
    desc: 'Convert images, audio, video and Office documents locally.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'organizer',
    icon: 'organizer',
    title: 'File Organizer',
    desc: 'Preview-first folder sorting with full one-click undo.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'backup',
    icon: 'backup',
    title: 'Backup',
    desc: 'Scheduled folder backups with versions, restore and a check that they are intact.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'clipboard',
    icon: 'clipboard',
    title: 'Clipboard',
    desc: 'Everything you copy, searchable and kept only on this computer. Pin what you reuse.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'text',
    icon: 'languages',
    title: 'Translator',
    desc: 'Translate any selected text with DeepL: replace it in place, or read it in a small window.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'phone',
    icon: 'smartphone',
    title: 'Phone transfer',
    desc: 'Send files between your phone and this computer over your Wi-Fi, with a QR code and no cloud.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'startup',
    icon: 'power',
    title: 'Startup apps',
    desc: 'See what opens with your computer and how much each one slowed it down. Turn off what you do not need.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'actions',
    icon: 'rocket',
    title: 'My actions',
    desc: 'Programs, folders, links and commands of your own, with shortcuts and a search box (Ctrl+K, or Alt+Space anywhere).',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'reminders',
    icon: 'bell',
    title: 'Reminders',
    desc: 'Say "in 20 minutes stretch" or "tomorrow at 9 call mom". A card appears when it is time.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'notes',
    icon: 'pencil',
    title: 'Notes',
    desc: 'Short notes kept on this computer. A shortcut opens a blank one at once.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'clocks',
    icon: 'clock',
    title: 'Time tools',
    desc: 'World clocks, a time-zone converter, a stopwatch and a timer that rings even with the window closed.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'toolbox',
    icon: 'wrench',
    title: 'Text toolbox',
    desc: 'JSON, Base64, URLs, hashes, UUIDs, regex, timestamps, text differences and colours, all on this computer.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'mini',
    icon: 'wand',
    title: 'Mini tools',
    desc: 'Keep a window on top, pick a colour from the screen, paste without formatting and clean tracking off links.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'inbox',
    icon: 'bell',
    title: 'Notifications',
    desc: 'The notices NomNom showed in the last days, so one that went away unread can be found again.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'rules',
    icon: 'wand',
    title: 'Folder rules',
    desc: 'When a file shows up in a folder, move it, rename it or turn it into WebP for you. Nothing is ever deleted, and every move can be undone.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'snippets',
    icon: 'file-text',
    title: 'Snippets',
    desc: 'Text you type again and again, saved under a name and pasted from the search, with the date or the clipboard filled in.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'shots',
    icon: 'image',
    title: 'Screenshots',
    desc: 'Take a picture of part of the screen and mark it up with arrows, boxes, numbers, text and blocked-out parts. Then copy or save it.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'gamemode',
    icon: 'playtime',
    title: 'Game mode',
    desc: 'While a game is in front: quieter notices, the fastest power plan and the background programs you choose closed. Put back afterwards.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'qr',
    icon: 'layers',
    title: 'QR codes',
    desc: 'Make a code from text or a Wi-Fi network, or read one that is on your screen.',
    tint: 'var(--glow-lavender)',
  },
  {
    id: 'compare',
    icon: 'arrow',
    title: 'Compare folders',
    desc: 'See what is different between two folders and copy what you choose. Nothing is deleted, and a replaced file keeps its old version.',
    tint: 'var(--glow-mint)',
  },
  {
    id: 'archives',
    icon: 'package',
    title: 'ZIP files',
    desc: 'Pack files and folders into a ZIP, or unpack one safely into a new folder.',
    tint: 'var(--glow-peach)',
  },
  {
    id: 'recorder',
    icon: 'film',
    title: 'Screen recorder',
    desc: 'Record the screen, or part of it, as a video or a small GIF. No sound.',
    tint: 'var(--glow-blush)',
  },
  {
    id: 'monitor',
    icon: 'activity',
    title: 'System Monitor',
    desc: 'Live processor, memory, disks, network and the programs using the most.',
    tint: 'var(--glow-mint)',
  },
];

/** Always in the sidebar: the pages that find and set up everything else. */
export const CORE = new Set(['home', 'tools', 'settings']);

/** What a new install starts with. The rest are one click away on the Tools page. */
export const DEFAULT_PICKS = ['downloads', 'activity', 'organizer', 'filetools'];

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
