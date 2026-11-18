import type { Motion, Theme } from '../../lib/ipc';

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
