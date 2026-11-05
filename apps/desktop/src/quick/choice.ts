import { formatBytes } from '../lib/format';
import type { Choice, MediaInfo } from '../lib/ipc';

export type QuickFormat = 'video' | 'm4a' | 'mp3';

export const FORMAT_OPTIONS: { value: QuickFormat; label: string }[] = [
  { value: 'video', label: 'Video' },
  { value: 'm4a', label: 'Audio · M4A' },
  { value: 'mp3', label: 'Audio · MP3' },
];

/** `best` until analysis finishes, then only resolutions the source actually offers. */
export function qualityOptions(
  format: QuickFormat,
  info: MediaInfo | null,
): { value: number | 'best'; label: string }[] {
  const best = { value: 'best' as const, label: 'Best available' };
  if (format !== 'video' || !info) return [best];
  return [
    best,
    ...info.heights.map((h) => {
      const size = formatBytes(info.size_by_height[String(h)]);
      return { value: h, label: size ? `${h}p · ${size}` : `${h}p` };
    }),
  ];
}

/** The defaults are Video and Best available, which need no analysis at all. */
export function buildChoice(format: QuickFormat, quality: number | 'best'): Choice {
  if (format === 'video') {
    return { kind: 'video', container: 'original', height: quality === 'best' ? null : quality };
  }
  return { kind: 'audio', container: format, height: null };
}
