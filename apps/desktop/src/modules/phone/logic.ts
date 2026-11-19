/** How long sharing may sit unused before it turns itself off. */
export const IDLE_CHOICES: { minutes: number; label: string }[] = [
  { minutes: 5, label: '5 minutes' },
  { minutes: 15, label: '15 minutes' },
  { minutes: 30, label: '30 minutes' },
  { minutes: 60, label: '1 hour' },
  { minutes: 240, label: '4 hours' },
];

/** The last part of a path, whichever slash it uses. */
export function fileNameOf(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}
