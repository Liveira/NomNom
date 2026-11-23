import { describe, expect, it } from 'vitest';
import {
  appBadgeColor,
  formatCallDuration,
  formatTrackTime,
  groupSmsThreads,
  relativeMinutes,
  timelineIcon,
} from './logic';

describe('mobile logic helpers', () => {
  it('groups SMS messages into sorted threads by latest timestamp', () => {
    const threads = groupSmsThreads([
      { id: '1', address: '+5511999990001', body: 'First', date_ms: 1000, incoming: true, read: true },
      { id: '2', address: '+5511999990002', body: 'Latest', date_ms: 3000, incoming: true, read: false },
      { id: '3', address: '+5511999990001', body: 'Reply', date_ms: 2000, incoming: false, read: true },
    ]);
    expect(threads.map((t) => t.address)).toEqual(['+5511999990002', '+5511999990001']);
    expect(threads[0].unread).toBe(1);
    expect(threads[1].messages.map((m) => m.body)).toEqual(['First', 'Reply']);
  });

  it('formats track times, call durations and relative times', () => {
    expect(formatTrackTime(215_000)).toBe('3:35');
    expect(formatTrackTime(0)).toBe('0:00');
    expect(formatCallDuration(87)).toBe('1m 27s');
    expect(formatCallDuration(42)).toBe('42s');
    expect(relativeMinutes(100_000, 100_000 + 18 * 60_000)).toBe('18 min');
  });

  it('maps timeline icons and deterministic badge colors', () => {
    expect(timelineIcon('mirror')).toBe('cast');
    expect(timelineIcon('clipboard')).toBe('clipboard');
    expect(appBadgeColor('WhatsApp')).toBe('#25D366');
    expect(appBadgeColor('Spotify')).toBe('#1DB954');
  });
});
