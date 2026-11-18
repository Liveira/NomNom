import type { ActRange } from '../../lib/ipc';

export interface CategoryInfo {
  id: string;
  label: string;
  color: string;
  tint: string;
}

export const CATEGORIES: CategoryInfo[] = [
  { id: 'creative', label: 'Creative', color: 'var(--blush)', tint: 'rgba(255, 143, 184, 0.16)' },
  { id: 'game', label: 'Games', color: 'var(--lavender)', tint: 'rgba(192, 163, 255, 0.16)' },
  { id: 'work', label: 'Work & Dev', color: 'var(--ok)', tint: 'rgba(123, 232, 188, 0.15)' },
  { id: 'browser', label: 'Browsers', color: 'var(--info)', tint: 'rgba(142, 197, 255, 0.16)' },
  { id: 'social', label: 'Social', color: 'var(--accent)', tint: 'rgba(255, 169, 112, 0.16)' },
  { id: 'media', label: 'Media', color: '#ff9a76', tint: 'rgba(255, 154, 118, 0.16)' },
  { id: 'general', label: 'General', color: 'var(--text-2)', tint: 'rgba(207, 194, 220, 0.12)' },
];

export const RANGES: { value: ActRange; label: string; title: string }[] = [
  { value: 'today', label: 'Today', title: "Today's active time" },
  { value: 'week', label: '7 Days', title: 'Last 7 days' },
  { value: 'month', label: '30 Days', title: 'Last 30 days' },
  { value: 'year', label: 'This Year', title: 'Last 365 days' },
  { value: 'all', label: 'All Time', title: 'All recorded time' },
];

export function categoryInfo(cat: string): CategoryInfo {
  return CATEGORIES.find((c) => c.id === cat) ?? CATEGORIES[CATEGORIES.length - 1];
}

export function formatHours(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  if (s === 0) return '0m';
  if (s < 60) return `${s}s`;
  const totalMins = Math.floor(s / 60);
  const h = Math.floor(totalMins / 60);
  const m = totalMins % 60;
  if (h === 0) return `${m}m`;
  if (m === 0) return `${h}h`;
  return `${h}h ${m}m`;
}

export function formatDecimalHours(secs: number): string {
  const s = Math.max(0, secs);
  if (s < 3600) return formatHours(s);
  const h = s / 3600;
  return h >= 100 ? `${Math.round(h)}h` : `${h.toFixed(1)}h`;
}

export function formatHourSlot(hour: number | null): string {
  if (hour == null || hour < 0 || hour > 23) return '—';
  const next = (hour + 1) % 24;
  const pad = (n: number) => String(n).padStart(2, '0');
  return `${pad(hour)}:00 – ${pad(next)}:00`;
}

export function relativeAgo(unixSecs: number, nowSecs = Math.floor(Date.now() / 1000)): string {
  if (!unixSecs) return 'Never';
  const diff = Math.max(0, nowSecs - unixSecs);
  if (diff < 90) return 'Just now';
  if (diff < 3600) return `${Math.floor(diff / 60)}m ago`;
  if (diff < 86400) return `${Math.floor(diff / 3600)}h ago`;
  if (diff < 172800) return 'Yesterday';
  if (diff < 86400 * 7) return `${Math.floor(diff / 86400)}d ago`;
  return new Date(unixSecs * 1000).toLocaleDateString([], { month: 'short', day: 'numeric' });
}

export function formatSessionSpan(startedAt: number, endedAt: number): string {
  const s = new Date(startedAt * 1000);
  const e = new Date(endedAt * 1000);
  const now = new Date();
  const sameDayAsNow =
    s.getFullYear() === now.getFullYear() &&
    s.getMonth() === now.getMonth() &&
    s.getDate() === now.getDate();
  const dayPart = sameDayAsNow
    ? 'Today'
    : s.toLocaleDateString([], { month: 'short', day: 'numeric' });
  const t1 = s.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  const t2 = e.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  return `${dayPart} · ${t1} – ${t2}`;
}

export function heatmapLevel(secs: number): 0 | 1 | 2 | 3 | 4 {
  if (secs <= 0) return 0;
  if (secs < 1800) return 1; // < 30m
  if (secs < 7200) return 2; // < 2h
  if (secs < 18000) return 3; // < 5h
  return 4; // 5h+
}

export function monogram(name: string): string {
  const parts = name
    .trim()
    .split(/[\s._-]+/)
    .filter(Boolean);
  if (parts.length >= 2) {
    return (parts[0][0] + parts[1][0]).toUpperCase();
  }
  return (name.trim().slice(0, 2) || '?').toUpperCase();
}
