/** What the File Tools screen shows and says. Pure, so it can be tested. */
import type { FtConversion, FtFile, FtKind, FtOptions, FtOutcome, FtPreset, FtTarget } from '../../lib/ipc';
import { formatBytes } from '../../lib/format';

export const KIND_NAMES: Record<FtKind, string> = {
  image: 'Picture',
  video: 'Video',
  audio: 'Sound',
  text: 'Text',
  office: 'Document',
};

export interface Controls {
  quality: boolean;
  videoQuality: boolean;
  size: boolean;
  fps: boolean;
  trim: boolean;
  audioKbps: boolean;
  metadata: boolean;
  background: boolean;
  pdfPage: boolean;
  combine: boolean;
}

const none: Controls = {
  quality: false, videoQuality: false, size: false, fps: false, trim: false,
  audioKbps: false, metadata: false, background: false, pdfPage: false, combine: false,
};

/** Which settings make sense for this result. Settings that would do nothing are not shown. */
export function controlsFor(target: FtTarget, kinds: FtKind[], fileCount: number): Controls {
  const allImages = kinds.length > 0 && kinds.every((k) => k === 'image');
  const hasVideo = kinds.includes('video');
  switch (target) {
    case 'jpg':
      return { ...none, quality: true, size: true, metadata: true, background: true };
    case 'webp':
      return { ...none, quality: true, size: true, metadata: true };
    case 'png':
      return { ...none, size: true, metadata: true };
    case 'bmp': case 'tiff': case 'gif': case 'ico':
      return { ...none, size: allImages };
    case 'pdf':
      return allImages
        ? { ...none, size: true, background: true, pdfPage: true, combine: fileCount > 1 }
        : none;
    case 'mp4': case 'webm': case 'mkv':
      return { ...none, videoQuality: target !== 'mkv', size: true, fps: true, trim: true };
    case 'no_audio':
      return none;
    case 'mp3': case 'm4a': case 'ogg': case 'opus':
      return { ...none, audioKbps: true, trim: true };
    case 'flac': case 'wav':
      return { ...none, trim: true };
    default:
      return hasVideo ? { ...none, trim: true } : none;
  }
}

/** Video to GIF needs its own small set. */
export function controlsForGif(kinds: FtKind[]): Controls {
  return kinds.includes('video') ? { ...none, size: true, fps: true, trim: true } : { ...none, size: true };
}

export function controls(target: FtTarget, kinds: FtKind[], fileCount: number): Controls {
  if (target === 'gif' && kinds.includes('video')) return controlsForGif(kinds);
  return controlsFor(target, kinds, fileCount);
}

/** A note shown before converting, when the choice can't give the user what they might expect. */
export function warningFor(files: Pick<FtFile, 'path' | 'kind'>[], target: FtTarget): string | null {
  const lossyAudio = /\.(mp3|m4a|aac|ogg|oga|opus|wma)$/i;
  if ((target === 'flac' || target === 'wav') && files.some((f) => lossyAudio.test(f.path))) {
    return "This won't improve quality. The source is already lossy.";
  }
  if (target === 'jpg' && files.some((f) => f.kind === 'image' && /\.(png|gif|webp|ico)$/i.test(f.path))) {
    return 'JPG has no transparency. Transparent areas, if there are any, are filled with the colour you choose.';
  }
  if (target === 'gif' && files.some((f) => f.kind === 'image')) {
    return 'GIF has at most 256 colours.';
  }
  if (target === 'pdf' && files.some((f) => f.kind === 'text')) {
    return "The PDF uses a built-in font, so only Western European letters can be shown.";
  }
  return null;
}

/** "1:30" or "90" or "1:02:03.5" to seconds. Empty means "not set". */
export function parseTime(text: string): number | null | 'bad' {
  const t = text.trim();
  if (t === '') return null;
  if (!/^\d+(:\d{1,2}){0,2}(\.\d+)?$/.test(t)) return 'bad';
  const parts = t.split(':').map(Number);
  let secs = 0;
  for (const p of parts) secs = secs * 60 + p;
  return Number.isFinite(secs) ? secs : 'bad';
}

/** Keeps a typed whole number inside a range; empty means "not set". */
export function parseCount(text: string, min: number, max: number): number | null | 'bad' {
  const t = text.trim();
  if (t === '') return null;
  if (!/^\d+$/.test(t)) return 'bad';
  const n = Number(t);
  return n >= min && n <= max ? n : 'bad';
}

/** Presets that fit the chosen files and can run right now. */
export function presetsFor(presets: FtPreset[], conversions: FtConversion[], kinds: FtKind[]): FtPreset[] {
  if (kinds.length === 0) return [];
  return presets.filter(
    (p) =>
      kinds.every((k) => k === p.kind) &&
      conversions.some((c) => c.target === p.target && c.kind === p.kind && c.available),
  );
}

export function mergeOptions(base: FtOptions, patch: Partial<FtOptions>): FtOptions {
  return { ...base, ...patch };
}

/** "3.2 MB to 1.1 MB (66% smaller)". */
export function savedLine(inBytes: number, outBytes: number): string {
  if (inBytes <= 0 || outBytes <= 0) return '';
  const base = `${formatBytes(inBytes)} to ${formatBytes(outBytes)}`;
  const change = Math.round((1 - outBytes / inBytes) * 100);
  if (change >= 1) return `${base} (${change}% smaller)`;
  if (change <= -1) return `${base} (${-change}% bigger)`;
  return base;
}

export function summaryLine(outcomes: FtOutcome[], targetLabel: string): string {
  const done = outcomes.filter((o) => o.status === 'done');
  const failed = outcomes.filter((o) => o.status === 'failed').length;
  const stopped = outcomes.filter((o) => o.status === 'cancelled').length;
  const doneSources = done.reduce((n, o) => n + o.sources, 0);
  let line: string;
  if (done.length === 0) line = 'Nothing was converted.';
  else if (done.length === 1 && doneSources > 1) line = `${doneSources} pictures combined into one ${targetLabel}.`;
  else line = `${done.length} ${done.length === 1 ? 'file' : 'files'} converted to ${targetLabel}.`;
  if (failed > 0) line += ` ${failed} failed.`;
  if (stopped > 0) line += ` ${stopped} stopped.`;
  return line;
}

export function fileName(path: string): string {
  return path.split(/[\\/]/).filter(Boolean).pop() ?? path;
}

/** Targets grouped for the picker, in a friendly order. */
export function groupTargets(conversions: FtConversion[]): { title: string; items: FtConversion[] }[] {
  const groups: { title: string; test: (c: FtConversion) => boolean }[] = [
    { title: 'Pictures', test: (c) => ['png', 'jpg', 'webp', 'bmp', 'tiff', 'ico'].includes(c.target) || (c.target === 'gif' && c.kind === 'image') },
    { title: 'Video', test: (c) => ['mp4', 'webm', 'mkv', 'no_audio'].includes(c.target) || (c.target === 'gif' && c.kind === 'video') },
    { title: 'Sound', test: (c) => ['mp3', 'm4a', 'flac', 'wav', 'ogg', 'opus'].includes(c.target) },
    { title: 'Documents', test: (c) => ['pdf', 'html'].includes(c.target) },
  ];
  return groups
    .map((g) => ({ title: g.title, items: conversions.filter(g.test) }))
    .filter((g) => g.items.length > 0);
}
