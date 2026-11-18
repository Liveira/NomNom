import type { BackupPlan } from '../../lib/ipc';

export const SCHEDULES: { id: BackupPlan['schedule']; label: string }[] = [
  { id: 'manual', label: 'Only when I press the button' },
  { id: 'hourly', label: 'Every hour' },
  { id: 'daily', label: 'Every day' },
  { id: 'weekly', label: 'Every week' },
];

export function newPlan(): BackupPlan {
  return { id: '', name: '', sources: [], destination: '', schedule: 'daily', keep: 10, exclude: 'node_modules, .git, *.tmp', enabled: true };
}

/** "5 minutes ago", "2 hours ago", "yesterday" for a time in Unix seconds. */
export function ago(secs: number, now: number = Date.now() / 1000): string {
  const d = Math.max(0, now - secs);
  if (d < 90) return 'just now';
  if (d < 3600) return `${Math.round(d / 60)} minutes ago`;
  if (d < 86_400) return `${Math.round(d / 3600)} hours ago`;
  if (d < 172_800) return 'yesterday';
  return `${Math.round(d / 86_400)} days ago`;
}

/** "in 5 minutes", "in 3 hours", or "now" for an upcoming time. */
export function inTime(secs: number, now: number = Date.now() / 1000): string {
  const d = secs - now;
  if (d <= 60) return 'now';
  if (d < 3600) return `in ${Math.round(d / 60)} minutes`;
  if (d < 86_400) return `in ${Math.round(d / 3600)} hours`;
  return `in ${Math.round(d / 86_400)} days`;
}

/** The last part of a path, for showing a folder by its name. */
export function leaf(path: string): string {
  const parts = path.split(/[\\/]+/).filter(Boolean);
  return parts[parts.length - 1] ?? path;
}
