/** The small text jobs: formats, encodings, hashes, colours, dates, differences and patterns. */

export type Result<T> = { ok: true; value: T } | { ok: false; error: string };
const ok = <T>(value: T): Result<T> => ({ ok: true, value });
const fail = <T>(error: string): Result<T> => ({ ok: false, error });

// ---- JSON ----

export function jsonFormat(text: string, indent = 2): Result<string> {
  try {
    return ok(JSON.stringify(JSON.parse(text), null, indent));
  } catch (e) {
    return fail(jsonProblem(text, e));
  }
}
export function jsonMinify(text: string): Result<string> {
  try {
    return ok(JSON.stringify(JSON.parse(text)));
  } catch (e) {
    return fail(jsonProblem(text, e));
  }
}
/** "Unexpected token at line 3, column 5": the message with a place in it. */
function jsonProblem(text: string, e: unknown): string {
  const pos = jsonErrorAt(text);
  const msg = e instanceof Error ? e.message.replace(/ in JSON at position \d+.*/, '').replace(/,\s*"[\s\S]*" is not valid JSON$/, '') : String(e);
  if (pos === null) return msg;
  const before = text.slice(0, pos).split('\n');
  return `${msg} (line ${before.length}, column ${before[before.length - 1].length + 1})`;
}

/** Where the first mistake in a JSON text is, or `null` when it is valid or the place can't be found. */
export function jsonErrorAt(text: string): number | null {
  let i = 0;
  const ws = () => { while (i < text.length && ' \t\n\r'.includes(text[i])) i++; };
  const bad = (): never => { throw i; };
  const lit = (w: string) => { if (text.startsWith(w, i)) i += w.length; else bad(); };
  const str = () => {
    i++;
    while (i < text.length && text[i] !== '"') {
      if (text[i] === '\\') i++;
      else if (text.charCodeAt(i) < 32) bad();
      i++;
    }
    if (text[i] !== '"') bad();
    i++;
  };
  const value = (): void => {
    ws();
    const c = text[i];
    if (c === '{') {
      i++; ws();
      if (text[i] === '}') { i++; return; }
      for (;;) {
        ws(); if (text[i] !== '"') bad();
        str(); ws(); if (text[i] !== ':') bad();
        i++; value(); ws();
        if (text[i] === ',') { i++; continue; }
        if (text[i] === '}') { i++; return; }
        bad();
      }
    } else if (c === '[') {
      i++; ws();
      if (text[i] === ']') { i++; return; }
      for (;;) {
        value(); ws();
        if (text[i] === ',') { i++; continue; }
        if (text[i] === ']') { i++; return; }
        bad();
      }
    } else if (c === '"') str();
    else if (c === 't') lit('true');
    else if (c === 'f') lit('false');
    else if (c === 'n') lit('null');
    else {
      const m = /^-?(0|[1-9]\d*)(\.\d+)?([eE][+-]?\d+)?/.exec(text.slice(i));
      if (!m) bad();
      i += m![0].length;
    }
  };
  try {
    value(); ws();
    if (i < text.length) bad();
    return null;
  } catch (p) {
    return typeof p === 'number' ? p : null;
  }
}

// ---- Base64 and URLs ----

export function base64Encode(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}
export function base64Decode(text: string): Result<string> {
  try {
    const clean = text.trim().replace(/-/g, '+').replace(/_/g, '/').replace(/\s+/g, '');
    const padded = clean + '='.repeat((4 - (clean.length % 4)) % 4);
    const bin = atob(padded);
    const bytes = Uint8Array.from(bin, (c) => c.charCodeAt(0));
    return ok(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch {
    return fail('That is not valid Base64 text.');
  }
}
export function urlEncode(text: string): string {
  return encodeURIComponent(text);
}
export function urlDecode(text: string): Result<string> {
  try {
    return ok(decodeURIComponent(text.replace(/\+/g, ' ')));
  } catch {
    return fail('That is not a valid encoded text.');
  }
}

// ---- Hashes ----

const hex = (buf: ArrayBuffer | Uint8Array) => [...new Uint8Array(buf instanceof Uint8Array ? buf.buffer.slice(buf.byteOffset, buf.byteOffset + buf.byteLength) : buf)].map((b) => b.toString(16).padStart(2, '0')).join('');

export async function digest(algo: 'SHA-1' | 'SHA-256' | 'SHA-384' | 'SHA-512', text: string): Promise<string> {
  return hex(await crypto.subtle.digest(algo, new TextEncoder().encode(text)));
}

/** MD5 (RFC 1321). Not for security: only to match a checksum somebody published. */
export function md5(text: string): string {
  const bytes = new TextEncoder().encode(text);
  const s = [7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21];
  const K = Array.from({ length: 64 }, (_, i) => Math.floor(Math.abs(Math.sin(i + 1)) * 2 ** 32) >>> 0);
  const len = bytes.length;
  const padded = new Uint8Array((((len + 8) >> 6) + 1) << 6);
  padded.set(bytes);
  padded[len] = 0x80;
  const view = new DataView(padded.buffer);
  view.setUint32(padded.length - 8, (len * 8) >>> 0, true);
  view.setUint32(padded.length - 4, Math.floor((len * 8) / 2 ** 32), true);
  let a0 = 0x67452301, b0 = 0xefcdab89, c0 = 0x98badcfe, d0 = 0x10325476;
  for (let off = 0; off < padded.length; off += 64) {
    const M = Array.from({ length: 16 }, (_, i) => view.getUint32(off + i * 4, true));
    let A = a0, B = b0, C = c0, D = d0;
    for (let i = 0; i < 64; i++) {
      let F: number, g: number;
      if (i < 16) { F = (B & C) | (~B & D); g = i; }
      else if (i < 32) { F = (D & B) | (~D & C); g = (5 * i + 1) % 16; }
      else if (i < 48) { F = B ^ C ^ D; g = (3 * i + 5) % 16; }
      else { F = C ^ (B | ~D); g = (7 * i) % 16; }
      F = (F + A + K[i] + M[g]) >>> 0;
      A = D; D = C; C = B;
      B = (B + ((F << s[i]) | (F >>> (32 - s[i])))) >>> 0;
    }
    a0 = (a0 + A) >>> 0; b0 = (b0 + B) >>> 0; c0 = (c0 + C) >>> 0; d0 = (d0 + D) >>> 0;
  }
  const out = new DataView(new ArrayBuffer(16));
  [a0, b0, c0, d0].forEach((v, i) => out.setUint32(i * 4, v, true));
  return hex(out.buffer);
}

// ---- Colours ----

export interface Rgb { r: number; g: number; b: number }

/** `#fa0`, `#ffaa00`, `rgb(255, 170, 0)`, `hsl(40, 100%, 50%)` or `255 170 0`. */
export function parseColor(text: string): Rgb | null {
  const t = text.trim().toLowerCase();
  let m = /^#?([0-9a-f]{3})$/.exec(t);
  if (m) return { r: parseInt(m[1][0] + m[1][0], 16), g: parseInt(m[1][1] + m[1][1], 16), b: parseInt(m[1][2] + m[1][2], 16) };
  m = /^#?([0-9a-f]{6})$/.exec(t);
  if (m) return { r: parseInt(m[1].slice(0, 2), 16), g: parseInt(m[1].slice(2, 4), 16), b: parseInt(m[1].slice(4, 6), 16) };
  m = /^(?:rgb\(\s*)?(\d{1,3})[\s,]+(\d{1,3})[\s,]+(\d{1,3})\s*\)?$/.exec(t);
  if (m) {
    const [r, g, b] = [Number(m[1]), Number(m[2]), Number(m[3])];
    return r < 256 && g < 256 && b < 256 ? { r, g, b } : null;
  }
  m = /^hsl\(\s*(\d{1,3})(?:deg)?[\s,]+(\d{1,3})%?[\s,]+(\d{1,3})%?\s*\)$/.exec(t);
  if (m) {
    const [h, s, l] = [Number(m[1]), Number(m[2]), Number(m[3])];
    return h <= 360 && s <= 100 && l <= 100 ? hslToRgb(h, s, l) : null;
  }
  return null;
}
export const toHex = ({ r, g, b }: Rgb): string => '#' + [r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('');
export function rgbToHsl({ r, g, b }: Rgb): { h: number; s: number; l: number } {
  const [R, G, B] = [r / 255, g / 255, b / 255];
  const max = Math.max(R, G, B), min = Math.min(R, G, B);
  const l = (max + min) / 2;
  if (max === min) return { h: 0, s: 0, l: Math.round(l * 100) };
  const d = max - min;
  const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
  const h = max === R ? (G - B) / d + (G < B ? 6 : 0) : max === G ? (B - R) / d + 2 : (R - G) / d + 4;
  return { h: Math.round(h * 60) % 360, s: Math.round(s * 100), l: Math.round(l * 100) };
}
export function hslToRgb(h: number, s: number, l: number): Rgb {
  const S = s / 100, L = l / 100;
  const k = (n: number) => (n + h / 30) % 12;
  const a = S * Math.min(L, 1 - L);
  const f = (n: number) => Math.round((L - a * Math.max(-1, Math.min(k(n) - 3, Math.min(9 - k(n), 1)))) * 255);
  return { r: f(0), g: f(8), b: f(4) };
}
/** WCAG contrast of two colours, 1 to 21. */
export function contrast(a: Rgb, b: Rgb): number {
  const lum = ({ r, g, b }: Rgb) => {
    const c = [r, g, b].map((v) => { const x = v / 255; return x <= 0.03928 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4; });
    return 0.2126 * c[0] + 0.7152 * c[1] + 0.0722 * c[2];
  };
  const [x, y] = [lum(a), lum(b)].sort((p, q) => q - p);
  return (x + 0.05) / (y + 0.05);
}

// ---- Time ----

/** A number of seconds or milliseconds since 1970, or an ISO date, as a moment. */
export function parseMoment(text: string): Date | null {
  const t = text.trim();
  if (/^-?\d{1,16}(\.\d+)?$/.test(t)) {
    const n = Number(t);
    // 12 digits and more are milliseconds; fewer are seconds.
    const ms = Math.abs(n) >= 1e11 ? n : n * 1000;
    const d = new Date(ms);
    return Number.isNaN(d.getTime()) ? null : d;
  }
  const d = new Date(t);
  return Number.isNaN(d.getTime()) ? null : d;
}

// ---- Differences ----

export interface DiffLine { kind: 'same' | 'add' | 'del'; text: string }

/** Line by line, by the longest common run. Inputs beyond a few thousand lines are refused. */
export function diffLines(a: string, b: string, limit = 3000): Result<DiffLine[]> {
  const x = a === '' ? [] : a.split('\n');
  const y = b === '' ? [] : b.split('\n');
  if (x.length > limit || y.length > limit) return fail(`That is too long to compare here (over ${limit} lines).`);
  const n = x.length, m = y.length;
  const dp = Array.from({ length: n + 1 }, () => new Uint32Array(m + 1));
  for (let i = n - 1; i >= 0; i--) for (let j = m - 1; j >= 0; j--) dp[i][j] = x[i] === y[j] ? dp[i + 1][j + 1] + 1 : Math.max(dp[i + 1][j], dp[i][j + 1]);
  const out: DiffLine[] = [];
  let i = 0, j = 0;
  while (i < n && j < m) {
    if (x[i] === y[j]) { out.push({ kind: 'same', text: x[i] }); i++; j++; }
    else if (dp[i + 1][j] >= dp[i][j + 1]) out.push({ kind: 'del', text: x[i++] });
    else out.push({ kind: 'add', text: y[j++] });
  }
  while (i < n) out.push({ kind: 'del', text: x[i++] });
  while (j < m) out.push({ kind: 'add', text: y[j++] });
  return ok(out);
}

// ---- Patterns ----

export interface Match { text: string; index: number; groups: string[] }

/** Runs a pattern on a text. The page does this in a worker with a time limit (see View). */
export function regexRun(pattern: string, flags: string, text: string, max = 200): Result<Match[]> {
  if (text.length > 100_000) return fail('That text is too long for this tester.');
  try {
    const re = new RegExp(pattern, flags.includes('g') ? flags : flags + 'g');
    const out: Match[] = [];
    for (const m of text.matchAll(re)) {
      out.push({ text: m[0], index: m.index ?? 0, groups: m.slice(1).map((g) => g ?? '') });
      if (out.length >= max) break;
    }
    return ok(out);
  } catch (e) {
    return fail(e instanceof Error ? e.message : String(e));
  }
}

// ---- Words ----

export type Case = 'upper' | 'lower' | 'title' | 'camel' | 'snake' | 'kebab';
const words = (t: string) => t.replace(/([a-z0-9])([A-Z])/g, '$1 $2').split(/[^\p{L}\p{N}]+/u).filter(Boolean);
export function convertCase(text: string, to: Case): string {
  switch (to) {
    case 'upper': return text.toUpperCase();
    case 'lower': return text.toLowerCase();
    case 'title': return text.toLowerCase().replace(/(^|[\s\-_(])(\p{L})/gu, (_, p, c) => p + c.toUpperCase());
    case 'camel': return words(text).map((w, i) => (i === 0 ? w.toLowerCase() : w[0].toUpperCase() + w.slice(1).toLowerCase())).join('');
    case 'snake': return words(text).map((w) => w.toLowerCase()).join('_');
    case 'kebab': return words(text).map((w) => w.toLowerCase()).join('-');
  }
}
export function countText(text: string): { chars: number; words: number; lines: number } {
  return { chars: [...text].length, words: text.trim() ? text.trim().split(/\s+/).length : 0, lines: text === '' ? 0 : text.split('\n').length };
}
