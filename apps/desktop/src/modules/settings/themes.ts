import type { Motion, Theme } from '../../lib/ipc';
import { isLight } from '../../lib/themes';

export interface ThemeInfo {
  id: Theme;
  name: string;
  tagline: string;
  icon: string;
}

/** The themes shown in Settings. "System" follows the operating system's dark or light setting. */
export const THEMES: ThemeInfo[] = [
  { id: 'system', name: 'System', tagline: 'Follows your computer: Midnight Mochi at night, Cream by day.', icon: 'monitor' },
  { id: 'dark', name: 'Midnight Mochi', tagline: 'Warm plum and peach. Soft lights drift behind.', icon: 'moon' },
  { id: 'light', name: 'Cream', tagline: 'Soft cream and peach, easy on the eyes in daylight.', icon: 'sun' },
  { id: 'aurora', name: 'Aurora', tagline: 'Northern lights flowing across deep indigo.', icon: 'sparkle' },
  { id: 'synthwave', name: 'Synthwave', tagline: 'Neon grid, a striped sun and scanlines.', icon: 'zap' },
  { id: 'sakura', name: 'Sakura', tagline: 'Cherry-blossom petals falling on soft pink.', icon: 'flower' },
  { id: 'ocean', name: 'Deep Ocean', tagline: 'Waves, rising bubbles and light from above.', icon: 'droplets' },
  { id: 'forest', name: 'Firefly Forest', tagline: 'Fireflies and mist among dark pines.', icon: 'trees' },
  { id: 'nebula', name: 'Nebula', tagline: 'Twinkling stars and shooting stars in a purple cosmos.', icon: 'orbit' },
  { id: 'sunset', name: 'Sunset', tagline: 'A low sun and drifting clouds at dusk.', icon: 'sunset' },
  { id: 'ember', name: 'Ember', tagline: 'Sparks rising from a glowing fire.', icon: 'flame' },
  { id: 'matrix', name: 'Matrix', tagline: 'Green code raining on black.', icon: 'terminal' },
  { id: 'cyberpunk', name: 'Cyberpunk', tagline: 'Neon bars and a flickering glitch.', icon: 'cpu' },
  { id: 'graphite', name: 'Graphite', tagline: 'Monochrome and calm, with slow beams of light.', icon: 'layers' },
  { id: 'matcha', name: 'Matcha', tagline: 'Soft green with leaves drifting down.', icon: 'leaf' },
  { id: 'lavender', name: 'Lavender', tagline: 'Pale violet with floating pollen.', icon: 'sparkle' },
  { id: 'frost', name: 'Frost', tagline: 'Icy blue with snow falling softly.', icon: 'droplets' },
  { id: 'candy', name: 'Candy', tagline: 'Bubblegum pink with floating bubbles.', icon: 'heart' },
];

export interface MotionInfo {
  id: Motion;
  name: string;
  hint: string;
  icon: string;
}

export const MOTIONS: MotionInfo[] = [
  { id: 'full', name: 'Full', hint: 'Animated background and interface', icon: 'sparkle' },
  { id: 'calm', name: 'Calm', hint: 'Still background, small interface motion', icon: 'leaf' },
  { id: 'off', name: 'Off', hint: 'Nothing moves', icon: 'pause' },
];

/** What the theme picker shows for a saved theme id. */
export function themeName(id: Theme): string {
  return THEMES.find((t) => t.id === id)?.name ?? id;
}

export interface ThemeGroup {
  label: string;
  items: ThemeInfo[];
}

/** The gallery sections, in the order they are shown: automatic, then dark, then light themes. */
export const THEME_GROUPS: ThemeGroup[] = [
  { label: 'Automatic', items: THEMES.filter((t) => t.id === 'system') },
  { label: 'Dark themes', items: THEMES.filter((t) => t.id !== 'system' && !isLight(t.id)) },
  { label: 'Light themes', items: THEMES.filter((t) => t.id !== 'system' && isLight(t.id)) },
];

/** All themes in gallery order, for keyboard movement. */
export const THEMES_IN_ORDER: ThemeInfo[] = THEME_GROUPS.flatMap((g) => g.items);
