/** Keeps the last `max` readings, newest last. */
export function pushSample(history: number[], value: number, max = 60): number[] {
  const next = [...history, Number.isFinite(value) ? value : 0];
  return next.length > max ? next.slice(next.length - max) : next;
}

/** Points for a small line chart, `w` by `h`, with values from 0 up to `top` (or the highest reading). */
export function sparkPoints(values: number[], w: number, h: number, top?: number): string {
  if (values.length < 2) return '';
  const max = Math.max(top ?? 0, ...values, 1e-9);
  const step = w / (values.length - 1);
  return values.map((v, i) => `${(i * step).toFixed(1)},${(h - (Math.min(v, max) / max) * h).toFixed(1)}`).join(' ');
}

/** "3 days 4 h", "2 h 15 min" or "8 min" for a stretch of seconds. */
export function uptimeText(secs: number): string {
  const d = Math.floor(secs / 86_400);
  const h = Math.floor((secs % 86_400) / 3600);
  const m = Math.floor((secs % 3600) / 60);
  if (d > 0) return `${d} ${d === 1 ? 'day' : 'days'} ${h} h`;
  if (h > 0) return `${h} h ${m} min`;
  return `${m} min`;
}

/** "1.2 MB/s" for bytes per second. */
export function rateText(bps: number): string {
  if (bps < 1024) return `${Math.round(bps)} B/s`;
  const units = ['KB', 'MB', 'GB'];
  let v = bps / 1024;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  return `${v >= 100 ? v.toFixed(0) : v.toFixed(1)} ${units[i]}/s`;
}

/** A colour name for how busy something is, so the screen can warn without relying on colour alone. */
export function load(percent: number): 'calm' | 'busy' | 'high' {
  return percent >= 90 ? 'high' : percent >= 70 ? 'busy' : 'calm';
}

export function percentOf(used: number, total: number): number {
  return total > 0 ? Math.min(100, Math.max(0, (used / total) * 100)) : 0;
}

/** "On battery: 3 h 20 min left", "Charging", "Plugged in". */
export function batteryLine(b: { plugged: boolean; charging: boolean; minutes_left: number | null }): string {
  if (b.charging) return 'Charging';
  if (b.plugged) return 'Plugged in';
  if (b.minutes_left == null) return 'On battery';
  const h = Math.floor(b.minutes_left / 60);
  const m = b.minutes_left % 60;
  return h > 0 ? `On battery: ${h} h ${m} min left` : `On battery: ${m} min left`;
}

/** A disk's hours of use as days: "52 days of use". */
export function useText(hours: number): string {
  const days = Math.round(hours / 24);
  return days >= 2 ? `${days} days of use` : `${hours} hours of use`;
}
