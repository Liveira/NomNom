export interface Section {
  id: string;
  name: string;
  icon: string;
  blurb: string;
  /** Words that find this section in the search box, besides its name. */
  words: string[];
}

export const SECTIONS: Section[] = [
  { id: 'appearance', name: 'Appearance', icon: 'palette', blurb: 'Themes, colours and how much things move.', words: ['theme', 'dark', 'light', 'colour', 'color', 'animation', 'motion', 'aurora', 'synthwave', 'sakura', 'ocean', 'forest', 'mochi', 'cream', 'background', 'reduce'] },
  { id: 'general', name: 'General', icon: 'sliders', blurb: 'How NomNom behaves when you close it.', words: ['tray', 'close', 'quit', 'background', 'exit'] },
  { id: 'downloads', name: 'Downloads', icon: 'download', blurb: 'Format, folder and tools for the downloader.', words: ['video', 'audio', 'format', 'quality', 'folder', 'yt-dlp', 'ffmpeg', 'cookies', 'duplicate', 'notify', 'save', 'subtitles', 'captions', 'schedule'] },
  { id: 'organizer', name: 'File Organizer', icon: 'organizer', blurb: 'Where sorted files go, and automatic sorting.', words: ['sort', 'folder', 'destination', 'automatic', 'conflict', 'downloads', 'move'] },
  { id: 'playtime', name: 'Playtime', icon: 'playtime', blurb: 'Active app and game hours.', words: ['tracking', 'hours', 'games', 'idle', 'afk', 'stats', 'apps'] },
  { id: 'browser', name: 'Browser', icon: 'globe', blurb: 'The Firefox extension and links from the web.', words: ['firefox', 'extension', 'link', 'add-on', 'web'] },
  { id: 'shortcuts', name: 'Shortcuts', icon: 'keyboard', blurb: 'Keys that work from anywhere.', words: ['keyboard', 'hotkey', 'key', 'quick', 'popup', 'ctrl', 'alt'] },
  { id: 'system', name: 'System', icon: 'monitor', blurb: 'Start-up and the right-click menu.', words: ['windows', 'linux', 'startup', 'start with', 'autostart', 'right-click', 'menu', 'explorer', 'integration', 'repair'] },
  { id: 'updates', name: 'Updates', icon: 'update', blurb: 'Keep NomNom up to date.', words: ['version', 'upgrade', 'release', 'check', 'new'] },
  { id: 'about', name: 'About', icon: 'info', blurb: 'Version, source code and credits.', words: ['licence', 'license', 'open source', 'credits', 'icons', 'fonts', 'github', 'lucide'] },
];

/** Sections that match what was typed (every word of it must appear in the name or the keywords). */
export function matchSections(query: string): Section[] {
  const terms = query.toLowerCase().split(/\s+/).filter(Boolean);
  if (terms.length === 0) return SECTIONS;
  return SECTIONS.filter((s) => {
    const hay = [s.name, s.blurb, ...s.words].join(' ').toLowerCase();
    return terms.every((t) => hay.includes(t));
  });
}
