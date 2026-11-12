import { formatBytes } from '../../lib/format';
import type { UpdateAvailable } from '../../lib/ipc';

/** "Last checked" line for the Updates section. `at` is Unix seconds, 0 when never. */
export function lastCheckedLine(at: number, now: number = Date.now() / 1000): string {
  if (!at) return 'Not checked yet.';
  const mins = Math.floor((now - at) / 60);
  if (mins < 1) return 'Checked just now.';
  if (mins < 60) return `Checked ${mins} minute${mins === 1 ? '' : 's'} ago.`;
  const hours = Math.floor(mins / 60);
  if (hours < 48) return `Checked ${hours} hour${hours === 1 ? '' : 's'} ago.`;
  return `Checked ${Math.floor(hours / 24)} days ago.`;
}

/** What the install button tells the user it will do. */
export function installLine(u: UpdateAvailable): string {
  const size = u.size > 0 ? ` (${formatBytes(u.size)})` : '';
  return `NomNom downloads the installer${size}, checks it against the checksum published with the release, closes, and starts again once it's installed.`;
}

/** Percent of the download done, or null when the size isn't known. */
export function percent(done: number, total: number | null): number | null {
  if (!total || total <= 0) return null;
  return Math.max(0, Math.min(100, Math.round((done / total) * 100)));
}
