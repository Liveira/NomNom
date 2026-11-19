/** Time zones, durations and the stopwatch's display. Everything uses the browser's own time zone data. */

export const DEFAULT_ZONES = ['UTC', 'America/New_York', 'America/Sao_Paulo', 'Europe/London', 'Europe/Madrid', 'Asia/Tokyo'];

export function localZone(): string {
  return Intl.DateTimeFormat().resolvedOptions().timeZone || 'UTC';
}

export function allZones(): string[] {
  try {
    const list = (Intl as unknown as { supportedValuesOf?: (k: string) => string[] }).supportedValuesOf?.('timeZone');
    if (list && list.length > 0) return list;
  } catch {
    /* an older engine */
  }
  return DEFAULT_ZONES;
}

export function isZone(tz: string): boolean {
  try {
    new Intl.DateTimeFormat('en', { timeZone: tz });
    return true;
  } catch {
    return false;
  }
}

/** How far ahead of UTC a zone is at a moment, in minutes (São Paulo is -180). */
export function offsetMinutes(tz: string, at: Date): number {
  const p = new Intl.DateTimeFormat('en-US', {
    timeZone: tz, hourCycle: 'h23', year: 'numeric', month: '2-digit', day: '2-digit', hour: '2-digit', minute: '2-digit', second: '2-digit',
  }).formatToParts(at);
  const get = (t: string) => Number(p.find((x) => x.type === t)?.value);
  const asUtc = Date.UTC(get('year'), get('month') - 1, get('day'), get('hour'), get('minute'), get('second'));
  return Math.round((asUtc - Math.floor(at.getTime() / 1000) * 1000) / 60000);
}

export function offsetLabel(minutes: number): string {
  const sign = minutes < 0 ? '-' : '+';
  const m = Math.abs(minutes);
  const h = Math.floor(m / 60);
  const rest = m % 60;
  return `UTC${sign}${h}${rest ? ':' + String(rest).padStart(2, '0') : ''}`;
}

/** "13:45" in a zone. */
export function clockIn(tz: string, at: Date, locale?: string): string {
  return at.toLocaleTimeString(locale, { timeZone: tz, hour: '2-digit', minute: '2-digit' });
}

/** "Fri, 20 Nov" in a zone. */
export function dateIn(tz: string, at: Date, locale?: string): string {
  return at.toLocaleDateString(locale, { timeZone: tz, weekday: 'short', day: 'numeric', month: 'short' });
}

/** The moment that shows `local` ("2026-11-19T14:30") on the wall clocks of `tz`. */
export function momentIn(local: string, tz: string): Date | null {
  const m = /^(\d{4})-(\d{2})-(\d{2})T(\d{2}):(\d{2})$/.exec(local);
  if (!m) return null;
  const wall = Date.UTC(Number(m[1]), Number(m[2]) - 1, Number(m[3]), Number(m[4]), Number(m[5]));
  // The offset depends on the moment itself (summer time), so it is settled in two steps.
  let guess = new Date(wall);
  for (let i = 0; i < 2; i++) guess = new Date(wall - offsetMinutes(tz, guess) * 60000);
  return Number.isNaN(guess.getTime()) ? null : guess;
}

/** "90" (minutes), "25m", "1h30", "1h 30m", "45s", "1:30:00" or "10:00" (minutes and seconds) as seconds. */
export function parseDuration(text: string): number | null {
  const t = text.trim().toLowerCase().replace(/\s+/g, '');
  if (!t) return null;
  let m = /^(\d+):(\d{1,2}):(\d{1,2})$/.exec(t);
  if (m) return Number(m[1]) * 3600 + Number(m[2]) * 60 + Number(m[3]);
  m = /^(\d+):(\d{1,2})$/.exec(t);
  if (m) return Number(m[1]) * 60 + Number(m[2]);
  m = /^(?:(\d+)h)?(?:(\d+)m(?:in)?)?(?:(\d+)s)?$/.exec(t);
  if (m && (m[1] || m[2] || m[3])) return Number(m[1] ?? 0) * 3600 + Number(m[2] ?? 0) * 60 + Number(m[3] ?? 0);
  m = /^(\d+)h(\d+)$/.exec(t);
  if (m) return Number(m[1]) * 3600 + Number(m[2]) * 60;
  if (/^\d+$/.test(t)) return Number(t) * 60;
  return null;
}

/** "1:05:09" for a countdown. */
export function clockText(seconds: number): string {
  const s = Math.max(0, Math.round(seconds));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const r = s % 60;
  const mm = String(m).padStart(2, '0');
  const ss = String(r).padStart(2, '0');
  return h > 0 ? `${h}:${mm}:${ss}` : `${mm}:${ss}`;
}

/** "01:05.42" for the stopwatch. */
export function stopwatchText(ms: number): string {
  const total = Math.max(0, Math.floor(ms));
  const h = Math.floor(total / 3_600_000);
  const m = Math.floor((total % 3_600_000) / 60_000);
  const s = Math.floor((total % 60_000) / 1000);
  const cs = Math.floor((total % 1000) / 10);
  const base = `${String(m).padStart(2, '0')}:${String(s).padStart(2, '0')}.${String(cs).padStart(2, '0')}`;
  return h > 0 ? `${h}:${base}` : base;
}
