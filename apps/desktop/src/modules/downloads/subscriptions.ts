/** How often NomNom looks for new videos. */
export const EVERY_CHOICES: { hours: number; label: string }[] = [
  { hours: 1, label: 'Every hour' },
  { hours: 3, label: 'Every 3 hours' },
  { hours: 6, label: 'Every 6 hours' },
  { hours: 12, label: 'Twice a day' },
  { hours: 24, label: 'Once a day' },
  { hours: 168, label: 'Once a week' },
];

/** "Looked 2 h ago" / "Looked just now" / "Not looked at yet". */
export function checkedText(lastChecked: number | null, everyHours: number, now: number = Math.floor(Date.now() / 1000)): string {
  if (lastChecked === null) return 'Not looked at yet';
  const s = Math.max(0, now - lastChecked);
  if (s < 90) return 'Looked just now';
  if (s < 3600) return `Looked ${Math.round(s / 60)} min ago`;
  if (s < 172_800) return `Looked ${Math.round(s / 3600)} h ago`;
  return `Looked ${Math.round(s / 86_400)} days ago, every ${everyHours} h`;
}
