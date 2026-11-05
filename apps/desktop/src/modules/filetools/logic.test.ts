import { describe, expect, it } from 'vitest';
import type { FtConversion, FtOptions, FtOutcome, FtPreset } from '../../lib/ipc';
import { controls, groupTargets, parseCount, parseTime, presetsFor, savedLine, summaryLine, warningFor } from './logic';

const conv = (kind: FtConversion['kind'], target: FtConversion['target'], available = true): FtConversion => ({
  kind, target, label: target.toUpperCase(), backend: 'image', lossiness: 'lossy', available, needs: available ? null : 'FFmpeg',
});
const outcome = (status: FtOutcome['status'], sources = 1): FtOutcome => ({
  src: 'C:\a.png', status, dst: status === 'done' ? 'C:\a.webp' : null, message: null, notes: [], in_bytes: 10, out_bytes: 5, sources,
});

describe('controls', () => {
  it('shows quality only where it does something', () => {
    expect(controls('jpg', ['image'], 1).quality).toBe(true);
    expect(controls('webp', ['image'], 1).quality).toBe(true);
    expect(controls('png', ['image'], 1).quality).toBe(false);
    expect(controls('flac', ['audio'], 1).audioKbps).toBe(false);
    expect(controls('mp3', ['audio'], 1).audioKbps).toBe(true);
  });
  it('offers trimming and frame rate for video results only', () => {
    expect(controls('mp4', ['video'], 1)).toMatchObject({ trim: true, fps: true, size: true, videoQuality: true });
    expect(controls('mkv', ['video'], 1).videoQuality).toBe(false);
    expect(controls('png', ['image'], 1).trim).toBe(false);
    expect(controls('no_audio', ['video'], 1)).toMatchObject({ trim: false, size: false, fps: false });
  });
  it('treats GIF from video differently from GIF from a picture', () => {
    expect(controls('gif', ['video'], 1)).toMatchObject({ fps: true, trim: true, size: true });
    expect(controls('gif', ['image'], 1)).toMatchObject({ fps: false, trim: false, size: true });
  });
  it('lets pictures be combined into one PDF only when there are several', () => {
    expect(controls('pdf', ['image'], 1).combine).toBe(false);
    expect(controls('pdf', ['image'], 3)).toMatchObject({ combine: true, pdfPage: true });
    expect(controls('pdf', ['text'], 3)).toMatchObject({ combine: false, pdfPage: false });
  });
});

describe('warnings', () => {
  it('warns when lossy sound goes to a lossless format', () => {
    expect(warningFor([{ path: 'C:\a.mp3', kind: 'audio' }], 'flac')).toContain("won't improve quality");
    expect(warningFor([{ path: 'C:\a.wav', kind: 'audio' }], 'flac')).toBeNull();
  });
  it('warns about transparency, colours and fonts', () => {
    expect(warningFor([{ path: 'C:\a.png', kind: 'image' }], 'jpg')).toContain('transparency');
    expect(warningFor([{ path: 'C:\a.jpg', kind: 'image' }], 'jpg')).toBeNull();
    expect(warningFor([{ path: 'C:\a.png', kind: 'image' }], 'gif')).toContain('256');
    expect(warningFor([{ path: 'C:\a.txt', kind: 'text' }], 'pdf')).toContain('Western European');
  });
});

describe('parsing typed values', () => {
  it('reads times', () => {
    expect(parseTime('')).toBeNull();
    expect(parseTime('90')).toBe(90);
    expect(parseTime('1:30')).toBe(90);
    expect(parseTime('1:02:03.5')).toBe(3723.5);
    expect(parseTime('abc')).toBe('bad');
    expect(parseTime('-5')).toBe('bad');
    expect(parseTime('1:2:3:4')).toBe('bad');
  });
  it('reads whole numbers inside a range', () => {
    expect(parseCount('', 1, 10)).toBeNull();
    expect(parseCount('5', 1, 10)).toBe(5);
    expect(parseCount('0', 1, 10)).toBe('bad');
    expect(parseCount('11', 1, 10)).toBe('bad');
    expect(parseCount('1.5', 1, 10)).toBe('bad');
  });
});

describe('presets', () => {
  const p = (id: string, kind: FtPreset['kind'], target: FtPreset['target']): FtPreset => ({
    id, name: id, description: '', kind, target, options: {} as FtOptions,
  });
  it('offers only presets that fit the files and can run', () => {
    const presets = [p('webp', 'image', 'webp'), p('mp3', 'audio', 'mp3'), p('web-video', 'video', 'mp4')];
    const convs = [conv('image', 'webp'), conv('audio', 'mp3', false), conv('video', 'mp4', false)];
    expect(presetsFor(presets, convs, ['image', 'image']).map((x) => x.id)).toEqual(['webp']);
    expect(presetsFor(presets, convs, ['audio'])).toEqual([]);
    expect(presetsFor(presets, convs, ['image', 'audio'])).toEqual([]);
    expect(presetsFor(presets, convs, [])).toEqual([]);
  });
});

describe('sentences', () => {
  it('describes size changes', () => {
    expect(savedLine(3_000_000, 1_000_000)).toBe('2.9 MB to 977 KB (67% smaller)');
    expect(savedLine(1000, 2000)).toBe('1000 B to 2.0 KB (100% bigger)');
    expect(savedLine(0, 10)).toBe('');
  });
  it('summarises a batch', () => {
    expect(summaryLine([outcome('done'), outcome('done')], 'WebP')).toBe('2 files converted to WebP.');
    expect(summaryLine([outcome('done')], 'WebP')).toBe('1 file converted to WebP.');
    expect(summaryLine([outcome('done'), outcome('failed')], 'WebP')).toBe('1 file converted to WebP. 1 failed.');
    expect(summaryLine([outcome('failed')], 'WebP')).toBe('Nothing was converted. 1 failed.');
    expect(summaryLine([outcome('done', 4)], 'PDF')).toBe('4 pictures combined into one PDF.');
    expect(summaryLine([outcome('cancelled'), outcome('cancelled')], 'MP3')).toBe('Nothing was converted. 2 stopped.');
  });
  it('groups targets for the picker', () => {
    const groups = groupTargets([conv('image', 'png'), conv('image', 'gif'), conv('image', 'pdf')]);
    expect(groups.map((g) => g.title)).toEqual(['Pictures', 'Documents']);
    expect(groups[0].items.map((i) => i.target)).toEqual(['png', 'gif']);
    const video = groupTargets([conv('video', 'mp4'), conv('video', 'gif'), conv('video', 'mp3')]);
    expect(video.map((g) => g.title)).toEqual(['Video', 'Sound']);
  });
});
