/** "just now", "5 min ago", "yesterday", "3 days ago", or the date for older ones. */
export function ago(unixSecs: number, now: number = Math.floor(Date.now() / 1000)): string {
  const s = Math.max(0, now - unixSecs);
  if (s < 45) return 'just now';
  if (s < 3600) return `${Math.max(1, Math.round(s / 60))} min ago`;
  if (s < 86_400) return `${Math.round(s / 3600)} h ago`;
  const days = Math.floor(s / 86_400);
  if (days === 1) return 'yesterday';
  if (days < 30) return `${days} days ago`;
  return new Date(unixSecs * 1000).toLocaleDateString();
}

/** How long entries are kept, for the choice in the page. 0 means for ever. */
export const KEEP_CHOICES: { days: number; label: string }[] = [
  { days: 1, label: '1 day' },
  { days: 7, label: '7 days' },
  { days: 30, label: '30 days' },
  { days: 90, label: '90 days' },
  { days: 365, label: '1 year' },
  { days: 0, label: 'For ever' },
];
