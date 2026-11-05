import { formatBytes } from '../lib/format';
import { defaultName } from '../lib/formats';
import type { Choice, Container, Kind, MediaInfo, Settings } from '../lib/ipc';

export type QuickFormat = Kind;

/** The two formats picked in Settings. Used when nothing has been loaded yet. */
export interface Defaults {
  kind: Kind;
  video: Container;
  audio: Container;
}

export const FALLBACK_DEFAULTS: Defaults = { kind: 'video', video: 'original', audio: 'original' };

export function defaultsFrom(settings: Settings | null | undefined): Defaults {
  const d = settings?.downloads;
  if (!d) return FALLBACK_DEFAULTS;
  return { kind: d.default_kind, video: d.default_video, audio: d.default_audio };
}

/** "Video · MP4" and "Audio · MP3": what each choice will really save as. */
export function formatOptions(d: Defaults): { value: QuickFormat; label: string }[] {
  return [
    { value: 'video', label: defaultName('video', d.video, d.audio) },
    { value: 'audio', label: defaultName('audio', d.video, d.audio) },
  ];
}

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

/** Video or audio in the format chosen in Settings, which needs no analysis to start. */
export function buildChoice(format: QuickFormat, quality: number | 'best', d: Defaults): Choice {
  if (format === 'video') {
    return { kind: 'video', container: d.video, height: quality === 'best' ? null : quality };
  }
  return { kind: 'audio', container: d.audio, height: null };
}
