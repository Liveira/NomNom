/** The formats a download can be saved as, with a plain explanation for each. */
import type { Container, Kind } from './ipc';

export interface FormatInfo {
  value: Container;
  label: string;
  /** One honest sentence about what choosing this means. */
  note: string;
}

// These lists must match VIDEO_FORMATS and AUDIO_FORMATS in modules/downloads.rs.
export const VIDEO_FORMATS: FormatInfo[] = [
  { value: 'original', label: 'Original', note: 'Keeps the file exactly as the site provides it. Nothing is converted.' },
  { value: 'mp4', label: 'MP4', note: 'Plays almost everywhere. Converted if the site uses something else.' },
  { value: 'webm', label: 'WebM', note: 'Small files that browsers like. Some players do not support it.' },
  { value: 'mkv', label: 'MKV', note: 'Holds any video and audio without converting. Some phones cannot play it.' },
];

export const AUDIO_FORMATS: FormatInfo[] = [
  { value: 'original', label: 'Original', note: 'Keeps the audio exactly as the site provides it. No conversion, no loss.' },
  { value: 'm4a', label: 'M4A', note: 'Plays almost everywhere. Usually saved without converting.' },
  { value: 'mp3', label: 'MP3', note: 'Works on every player. The audio is converted, so it loses a little.' },
  { value: 'opus', label: 'Opus', note: 'Small files with good quality. Some older players cannot play it.' },
  { value: 'ogg', label: 'OGG', note: 'An open format. The audio is converted.' },
  { value: 'flac', label: 'FLAC', note: 'Lossless file, but it cannot add quality the source never had.' },
  { value: 'wav', label: 'WAV', note: 'Uncompressed and very large. It cannot add quality either.' },
];

export function formatsFor(kind: Kind): FormatInfo[] {
  return kind === 'video' ? VIDEO_FORMATS : AUDIO_FORMATS;
}

export function formatLabel(kind: Kind, value: string): string {
  return formatsFor(kind).find((f) => f.value === value)?.label ?? 'Original';
}

export function formatNote(kind: Kind, value: string): string {
  return formatsFor(kind).find((f) => f.value === value)?.note ?? '';
}

/** "Video · MP4" or "Audio · Original": the short name shown wherever a default is mentioned. */
export function defaultName(kind: Kind, video: string, audio: string): string {
  return kind === 'video'
    ? `Video · ${formatLabel('video', video)}`
    : `Audio · ${formatLabel('audio', audio)}`;
}
