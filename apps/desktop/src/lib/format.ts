export function formatBytes(n: number | null | undefined): string {
  if (n == null || !Number.isFinite(n) || n < 0) return '';
  const units = ['B', 'KB', 'MB', 'GB', 'TB'];
  let v = n;
  let i = 0;
  while (v >= 1024 && i < units.length - 1) {
    v /= 1024;
    i++;
  }
  const digits = i === 0 || v >= 100 ? 0 : 1;
  return `${v.toFixed(digits)} ${units[i]}`;
}

export function formatDuration(secs: number | null | undefined): string {
  if (secs == null || secs < 0) return '';
  const h = Math.floor(secs / 3600);
  const m = Math.floor((secs % 3600) / 60);
  const s = Math.floor(secs % 60);
  const two = (n: number) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${two(m)}:${two(s)}` : `${m}:${two(s)}`;
}

export function formatSpeed(bps: number | null | undefined): string {
  return bps == null || bps <= 0 ? '' : `${formatBytes(bps)}/s`;
}

export function formatEta(secs: number | null | undefined): string {
  if (secs == null || secs < 0) return '';
  if (secs < 60) return `${Math.max(1, Math.round(secs))} s left`;
  if (secs < 3600) return `${Math.round(secs / 60)} min left`;
  return `${Math.floor(secs / 3600)} h ${Math.round((secs % 3600) / 60)} min left`;
}

export function heightLabel(h: number): string {
  return `${h}p`;
}

/** "62% · 2.3 MB/s · 4 s left": only the parts that are known, joined with a middle dot. */
export function progressLine(p: {
  fraction: number | null;
  speedBps: number | null;
  etaSecs: number | null;
  etaReliable: boolean;
}): string {
  const parts = [
    p.fraction != null ? `${Math.round(p.fraction * 100)}%` : 'Starting',
    formatSpeed(p.speedBps),
    p.etaReliable ? formatEta(p.etaSecs) : '',
  ];
  return parts.filter(Boolean).join(' · ');
}
