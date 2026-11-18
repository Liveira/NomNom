/** mm:ss for a countdown, or h:mm:ss when it is an hour or more. */
export function clock(totalSecs: number): string {
  const s = Math.max(0, Math.floor(totalSecs));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const two = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${two(m)}:${two(sec)}` : `${two(m)}:${two(sec)}`;
}

/** Length of the ring that is empty, given the circumference and the fraction of time still left. */
export function ringOffset(circumference: number, fractionLeft: number): number {
  const f = Math.min(1, Math.max(0, fractionLeft));
  return circumference * (1 - f);
}

/** "25m", "1h 5m": minutes and hours for a stretch of seconds. */
export function formatHours(secs: number): string {
  const s = Math.max(0, Math.floor(secs));
  if (s < 60) return s === 0 ? '0m' : `${s}s`;
  const m = Math.floor(s / 60);
  const h = Math.floor(m / 60);
  if (h === 0) return `${m}m`;
  return m % 60 === 0 ? `${h}h` : `${h}h ${m % 60}m`;
}
