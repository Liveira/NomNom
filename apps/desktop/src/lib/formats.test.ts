import { describe, expect, it } from 'vitest';
import { AUDIO_FORMATS, VIDEO_FORMATS, defaultName, formatLabel, formatNote } from './formats';

describe('format lists', () => {
  it('offer Original first for both video and audio', () => {
    expect(VIDEO_FORMATS[0].value).toBe('original');
    expect(AUDIO_FORMATS[0].value).toBe('original');
  });

  it('video formats are exactly the ones the backend accepts', () => {
    expect(VIDEO_FORMATS.map((f) => f.value)).toEqual(['original', 'mp4', 'webm', 'mkv']);
  });

  it('audio formats are exactly the ones the backend accepts', () => {
    expect(AUDIO_FORMATS.map((f) => f.value)).toEqual(['original', 'm4a', 'mp3', 'opus', 'ogg', 'flac', 'wav']);
  });

  it('never claim that converting improves quality', () => {
    for (const f of [...VIDEO_FORMATS, ...AUDIO_FORMATS]) {
      expect(f.note.toLowerCase()).not.toMatch(/better quality|improve|enhance/);
    }
    expect(formatNote('audio', 'flac')).toContain('cannot add quality');
    expect(formatNote('audio', 'wav')).toContain('cannot add quality');
  });

  it('every format has a short label and an explanation', () => {
    for (const f of [...VIDEO_FORMATS, ...AUDIO_FORMATS]) {
      expect(f.label.length).toBeGreaterThan(0);
      expect(f.note.length).toBeGreaterThan(10);
    }
  });
});

describe('names shown for the defaults', () => {
  it('say which kind they apply to', () => {
    expect(defaultName('video', 'mp4', 'mp3')).toBe('Video · MP4');
    expect(defaultName('audio', 'mp4', 'mp3')).toBe('Audio · MP3');
    expect(defaultName('audio', 'original', 'original')).toBe('Audio · Original');
  });

  it('fall back to Original for anything unknown', () => {
    expect(formatLabel('video', 'banana')).toBe('Original');
    expect(formatLabel('video', 'mp3')).toBe('Original');
  });
});
