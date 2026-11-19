import { describe, expect, it } from 'vitest';
import { base64Decode, base64Encode, contrast, convertCase, countText, diffLines, digest, hslToRgb, jsonFormat, jsonMinify, md5, parseColor, parseMoment, regexRun, rgbToHsl, toHex, urlDecode, urlEncode } from './logic';

describe('JSON', () => {
  it('formats and minifies', () => {
    const f = jsonFormat('{"a":[1,2],"b":{"c":null}}');
    expect(f.ok && f.value).toBe('{\n  "a": [\n    1,\n    2\n  ],\n  "b": {\n    "c": null\n  }\n}');
    const m = jsonMinify('{ "a": [ 1, 2 ] }');
    expect(m.ok && m.value).toBe('{"a":[1,2]}');
  });
  it('says where it broke', () => {
    const r = jsonFormat('{\n  "a": 1,\n  "b": }');
    expect(r.ok).toBe(false);
    if (!r.ok) expect(r.error).toMatch(/line 3/);
  });
});

describe('Base64 and URL', () => {
  it('round-trips text with accents and emoji', () => {
    const t = 'Olá, mundo — 🐱';
    const e = base64Encode(t);
    expect(e).toBe('T2zDoSwgbXVuZG8g4oCUIPCfkLE=');
    const d = base64Decode(e);
    expect(d.ok && d.value).toBe(t);
  });
  it('reads the URL-safe form and missing padding', () => {
    const d = base64Decode('T2zDoSwgbXVuZG8g4oCUIPCfkLE');
    expect(d.ok && d.value).toBe('Olá, mundo — 🐱');
  });
  it('refuses nonsense', () => {
    expect(base64Decode('%%%').ok).toBe(false);
    expect(urlDecode('%E0%A4%A').ok).toBe(false);
  });
  it('encodes and decodes URL text', () => {
    expect(urlEncode('a b&c=d/é')).toBe('a%20b%26c%3Dd%2F%C3%A9');
    const d = urlDecode('a%20b%26c+d');
    expect(d.ok && d.value).toBe('a b&c d');
  });
});

describe('hashes', () => {
  it('matches the published MD5 test vectors', () => {
    expect(md5('')).toBe('d41d8cd98f00b204e9800998ecf8427e');
    expect(md5('abc')).toBe('900150983cd24fb0d6963f7d28e17f72');
    expect(md5('The quick brown fox jumps over the lazy dog')).toBe('9e107d9d372bb6826bd81d3542a419d6');
    expect(md5('a'.repeat(100))).toBe('36a92cc94a9e0fa21f625f8bfb007adf');
  });
  it('matches the SHA test vectors', async () => {
    expect(await digest('SHA-1', 'abc')).toBe('a9993e364706816aba3e25717850c26c9cd0d89d');
    expect(await digest('SHA-256', 'abc')).toBe('ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad');
  });
});

describe('colours', () => {
  it('reads the usual spellings', () => {
    expect(parseColor('#fa0')).toEqual({ r: 255, g: 170, b: 0 });
    expect(parseColor('FFAA00')).toEqual({ r: 255, g: 170, b: 0 });
    expect(parseColor('rgb(255, 170, 0)')).toEqual({ r: 255, g: 170, b: 0 });
    expect(parseColor('255 170 0')).toEqual({ r: 255, g: 170, b: 0 });
    expect(parseColor('hsl(40, 100%, 50%)')).toEqual({ r: 255, g: 170, b: 0 });
  });
  it('turns away what is not a colour', () => {
    expect(parseColor('#gg0000')).toBeNull();
    expect(parseColor('rgb(300, 0, 0)')).toBeNull();
    expect(parseColor('hsl(400, 10%, 10%)')).toBeNull();
    expect(parseColor('')).toBeNull();
  });
  it('converts both ways', () => {
    expect(toHex({ r: 255, g: 170, b: 0 })).toBe('#ffaa00');
    expect(rgbToHsl({ r: 255, g: 170, b: 0 })).toEqual({ h: 40, s: 100, l: 50 });
    expect(hslToRgb(0, 100, 50)).toEqual({ r: 255, g: 0, b: 0 });
    expect(rgbToHsl({ r: 128, g: 128, b: 128 })).toEqual({ h: 0, s: 0, l: 50 });
  });
  it('measures contrast', () => {
    expect(contrast({ r: 0, g: 0, b: 0 }, { r: 255, g: 255, b: 255 })).toBeCloseTo(21, 0);
    expect(contrast({ r: 255, g: 255, b: 255 }, { r: 255, g: 255, b: 255 })).toBeCloseTo(1, 5);
  });
});

describe('moments', () => {
  it('tells seconds from milliseconds', () => {
    expect(parseMoment('1700000000')?.toISOString()).toBe('2023-11-14T22:13:20.000Z');
    expect(parseMoment('1700000000000')?.toISOString()).toBe('2023-11-14T22:13:20.000Z');
  });
  it('reads a date and refuses words', () => {
    expect(parseMoment('2023-11-14T22:13:20Z')?.getTime()).toBe(1700000000000);
    expect(parseMoment('yesterday-ish')).toBeNull();
  });
});

describe('diff', () => {
  it('marks added, removed and unchanged lines', () => {
    const r = diffLines('a\nb\nc', 'a\nc\nd');
    expect(r.ok && r.value.map((l) => l.kind[0] + l.text).join(',')).toBe('sa,db,sc,ad');
  });
  it('treats empty text as no lines', () => {
    const r = diffLines('', 'x');
    expect(r.ok && r.value).toEqual([{ kind: 'add', text: 'x' }]);
  });
  it('refuses a huge input instead of freezing', () => {
    expect(diffLines('x\n'.repeat(4000), 'y', 3000).ok).toBe(false);
  });
});

describe('patterns', () => {
  it('finds every match with its groups', () => {
    const r = regexRun('(\\d+)-(\\w)', '', 'a 12-x b 7-y');
    expect(r.ok && r.value.map((m) => [m.text, m.index, ...m.groups])).toEqual([['12-x', 2, '12', 'x'], ['7-y', 9, '7', 'y']]);
  });
  it('explains a broken pattern', () => {
    const r = regexRun('(', '', 'x');
    expect(r.ok).toBe(false);
  });
  it('does not loop on an empty match', () => {
    const r = regexRun('x*', '', 'abc');
    expect(r.ok && r.value.length).toBeGreaterThan(0);
  });
});

describe('words', () => {
  it('converts the case of a phrase', () => {
    expect(convertCase('hello big World', 'title')).toBe('Hello Big World');
    expect(convertCase('hello big World', 'camel')).toBe('helloBigWorld');
    expect(convertCase('helloBigWorld', 'snake')).toBe('hello_big_world');
    expect(convertCase('Hello, big World!', 'kebab')).toBe('hello-big-world');
    expect(convertCase('abc', 'upper')).toBe('ABC');
  });
  it('counts', () => {
    expect(countText('')).toEqual({ chars: 0, words: 0, lines: 0 });
    expect(countText('two words\nnext')).toEqual({ chars: 14, words: 3, lines: 2 });
  });
});
