import type { MobileSms } from '../../lib/ipc';

export type MobileTab =
  | 'home'
  | 'messages'
  | 'calls'
  | 'photos'
  | 'apps'
  | 'deck'
  | 'guide'
  | 'settings';

export interface SmsThread {
  address: string;
  lastBody: string;
  lastDateMs: number;
  unread: number;
  messages: MobileSms[];
}

export function groupSmsThreads(items: MobileSms[]): SmsThread[] {
  const map = new Map<string, SmsThread>();
  for (const m of items) {
    const key = m.address.trim();
    if (!key) continue;
    let t = map.get(key);
    if (!t) {
      t = { address: key, lastBody: m.body, lastDateMs: m.date_ms, unread: 0, messages: [] };
      map.set(key, t);
    }
    t.messages.push(m);
    if (m.date_ms > t.lastDateMs) {
      t.lastDateMs = m.date_ms;
      t.lastBody = m.body;
    }
    if (!m.read && m.incoming) t.unread += 1;
  }
  const out = [...map.values()].sort((a, b) => b.lastDateMs - a.lastDateMs);
  for (const t of out) {
    t.messages.sort((a, b) => a.date_ms - b.date_ms);
  }
  return out;
}

export function relativeMinutes(epochMs: number, nowMs = Date.now()): string {
  if (!epochMs) return 'now';
  const diff = Math.max(0, Math.floor((nowMs - epochMs) / 1000));
  if (diff < 60) return 'now';
  if (diff < 3600) return `${Math.floor(diff / 60)} min`;
  if (diff < 86400) return `${Math.floor(diff / 3600)} h`;
  return `${Math.floor(diff / 86400)} d`;
}

export function relativeSecs(epochSecs: number, nowSecs = Math.floor(Date.now() / 1000)): string {
  return relativeMinutes(epochSecs * 1000, nowSecs * 1000);
}

export function formatTrackTime(ms: number): string {
  const total = Math.max(0, Math.floor(ms / 1000));
  const m = Math.floor(total / 60);
  const s = total % 60;
  return `${m}:${String(s).padStart(2, '0')}`;
}

export function formatCallDuration(secs: number): string {
  if (secs <= 0) return '0s';
  if (secs < 60) return `${secs}s`;
  const m = Math.floor(secs / 60);
  const s = secs % 60;
  return s > 0 ? `${m}m ${s}s` : `${m}m`;
}

export function timelineIcon(kind: string): string {
  switch (kind) {
    case 'mirror':
      return 'cast';
    case 'clipboard':
      return 'clipboard';
    case 'file':
      return 'send';
    case 'camera':
    case 'photo':
      return 'camera';
    case 'webcam':
      return 'video';
    case 'audio':
      return 'volume';
    case 'call':
      return 'phone';
    case 'message':
      return 'message';
    case 'find':
      return 'bell-ring';
    case 'app':
      return 'grid';
    case 'pair':
    case 'connect':
      return 'wifi';
    default:
      return 'smartphone';
  }
}

export function appBadgeColor(pkgOrName: string): string {
  const s = pkgOrName.toLowerCase();
  if (s.includes('whatsapp')) return '#25D366';
  if (s.includes('spotify')) return '#1DB954';
  if (s.includes('telegram')) return '#26A5E4';
  if (s.includes('discord')) return '#5865F2';
  if (s.includes('youtube')) return '#FF0033';
  if (s.includes('instagram')) return '#E1306C';
  if (s.includes('gmail')) return '#EA4335';
  if (s.includes('grok') || s.includes('twitter')) return '#7C8594';
  let hash = 0;
  for (let i = 0; i < s.length; i++) hash = (hash * 31 + s.charCodeAt(i)) | 0;
  const palette = ['#e8836b', '#5ea8ff', '#58c996', '#b88aff', '#f2a65a', '#e879a9'];
  return palette[Math.abs(hash) % palette.length];
}
