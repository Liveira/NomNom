/** Palettes whose colours are light. Everything else is dark. Used for the browser's colour scheme. */
export const LIGHT_PALETTES = new Set(['light', 'sakura', 'matcha', 'lavender', 'frost', 'candy']);
export const isLight = (palette: string): boolean => LIGHT_PALETTES.has(palette);

/** Full-app architectural layout variants driven by the active theme. */
export type ThemeLayout = 'studio' | 'floating' | 'dock-top' | 'rail' | 'cyber-hud' | 'terminal';

const THEME_LAYOUTS: Record<string, ThemeLayout> = {
  dark: 'studio',
  light: 'studio',
  graphite: 'studio',
  sakura: 'floating',
  candy: 'floating',
  lavender: 'floating',
  matcha: 'floating',
  aurora: 'dock-top',
  ocean: 'dock-top',
  frost: 'dock-top',
  nebula: 'rail',
  sunset: 'rail',
  ember: 'rail',
  forest: 'rail',
  cyberpunk: 'cyber-hud',
  synthwave: 'cyber-hud',
  matrix: 'terminal',
};

export const LAYOUT_LABELS: Record<ThemeLayout, string> = {
  studio: 'Classic Studio',
  floating: 'Floating · Pill',
  'dock-top': 'Top Bar · Glass',
  rail: 'Icon Rail · Bento',
  'cyber-hud': 'Right HUD · Sharp',
  terminal: 'Terminal TUI',
};

export const themeLayout = (palette: string): ThemeLayout => THEME_LAYOUTS[palette] ?? 'studio';
