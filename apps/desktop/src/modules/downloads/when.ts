/** "at 21:30 today", "at 08:00 tomorrow", or the date, for a time given in Unix seconds. */
export function whenText(unixSecs: number, now: Date = new Date()): string {
  const d = new Date(unixSecs * 1000);
  const time = d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  const startOfDay = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const days = Math.round((startOfDay(d) - startOfDay(now)) / 86_400_000);
  if (days === 0) return `at ${time} today`;
  if (days === 1) return `at ${time} tomorrow`;
  return `on ${d.toLocaleDateString([], { month: 'short', day: 'numeric' })} at ${time}`;
}

/** Unix seconds for the value of a datetime-local input ("2026-11-19T21:30"), or null if it isn't one. */
export function parseLocal(value: string): number | null {
  if (!/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(value)) return null;
  const t = new Date(value).getTime();
  return Number.isNaN(t) ? null : Math.floor(t / 1000);
}

/** A datetime-local value for `secs` from now, rounded up to the next 5 minutes. */
export function suggestLocal(now: Date = new Date(), hours = 1): string {
  const t = new Date(now.getTime() + hours * 3_600_000);
  t.setMinutes(Math.ceil(t.getMinutes() / 5) * 5, 0, 0);
  const p = (n: number) => String(n).padStart(2, '0');
  return `${t.getFullYear()}-${p(t.getMonth() + 1)}-${p(t.getDate())}T${p(t.getHours())}:${p(t.getMinutes())}`;
}
