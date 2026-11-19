/** Words for a moment in time, for the reminders list and the search box. */

const MIN = 60;
const HOUR = 3600;

/** "in 12 min", "in 3 h", or the day and time when it is further away. `now` and `due` are unix seconds. */
export function dueText(due: number, now: number, locale?: string): string {
  const wait = due - now;
  if (wait <= 0) return 'now';
  if (wait < MIN) return 'in under a minute';
  if (wait < HOUR) return `in ${Math.round(wait / MIN)} min`;
  const d = new Date(due * 1000);
  const n = new Date(now * 1000);
  const time = d.toLocaleTimeString(locale, { hour: '2-digit', minute: '2-digit' });
  if (wait < 6 * HOUR) {
    const h = Math.floor(wait / HOUR);
    const m = Math.round((wait % HOUR) / MIN);
    return m === 0 ? `in ${h} h` : `in ${h} h ${m} min`;
  }
  if (sameDay(d, n)) return `today at ${time}`;
  const tomorrow = new Date(n);
  tomorrow.setDate(n.getDate() + 1);
  if (sameDay(d, tomorrow)) return `tomorrow at ${time}`;
  const date = d.toLocaleDateString(locale, { weekday: 'short', day: 'numeric', month: 'short' });
  return `${date}, ${time}`;
}

function sameDay(a: Date, b: Date): boolean {
  return a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();
}

/** A reminder typed into the search box starts with one of these words (English, Portuguese, Spanish). */
export function startsLikeReminder(text: string): boolean {
  return /^\s*(remind|reminder|me lembr|lembr|lembrete|recu[eé]rd|recordat|recordar)/i.test(text);
}

/** A note typed into the search box starts with one of these words. Returns the note, or `null`. */
export function noteFrom(text: string): string | null {
  const m = /^\s*(?:note|nota|anotar|anota|apunte)\s*[:\-]?\s+(\S[\s\S]*)$/i.exec(text);
  return m ? m[1].trim() : null;
}

export const EXAMPLES = ['in 20 minutes stretch', 'tomorrow at 9am call mom', 'every day at 18:30 drink water', 'em 2 horas tirar o bolo'];

export const SNOOZES: { label: string; minutes: number }[] = [
  { label: '+10 min', minutes: 10 },
  { label: '+1 h', minutes: 60 },
  { label: 'Tomorrow', minutes: 24 * 60 },
];
