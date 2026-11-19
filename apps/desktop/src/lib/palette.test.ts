import { describe, expect, it } from 'vitest';
import { calc, lastTwo, rank, score, showNumber } from './palette';

describe('score', () => {
  it('an empty query matches everything', () => {
    expect(score('', 'anything')).toBeGreaterThan(0);
  });
  it('prefers the start of the text, then the start of a word, then the middle', () => {
    const start = score('dow', 'Downloader');
    const word = score('dow', 'My Downloader');
    const middle = score('own', 'Downloader');
    expect(start).toBeGreaterThan(word);
    expect(word).toBeGreaterThan(middle);
    expect(middle).toBeGreaterThan(0);
  });
  it('finds scattered letters in order and nothing out of order', () => {
    expect(score('vsc', 'Visual Studio Code')).toBeGreaterThan(0);
    expect(score('cvs', 'Visual Studio Code')).toBe(0);
  });
  it('every word of the query has to match', () => {
    expect(score('disk an', 'Disk Analyzer')).toBeGreaterThan(0);
    expect(score('disk zebra', 'Disk Analyzer')).toBe(0);
  });
  it('ignores case', () => {
    expect(score('PLAY', 'playtime')).toBeGreaterThan(0);
  });
});

describe('rank', () => {
  const items = [
    { title: 'Settings' },
    { title: 'Startup apps' },
    { title: 'Theme: Aurora', keywords: 'colour palette' },
    { title: 'Set the volume' },
  ];
  it('puts the closest match first', () => {
    expect(rank(items, 'set')[0].title).toBe('Settings');
  });
  it('drops what does not match', () => {
    expect(rank(items, 'zzz')).toEqual([]);
  });
  it('can find an item by its extra words', () => {
    expect(rank(items, 'colour').map((i) => i.title)).toEqual(['Theme: Aurora']);
  });
  it('shows the first ones when nothing is typed, up to the limit', () => {
    expect(rank(items, '  ', 2).map((i) => i.title)).toEqual(['Settings', 'Startup apps']);
  });
});

describe('calc', () => {
  it('does ordinary arithmetic with the usual order', () => {
    expect(calc('2 + 3 * 4')).toBe(14);
    expect(calc('(2 + 3) * 4')).toBe(20);
    expect(calc('2 ^ 3 ^ 2')).toBe(512);
    expect(calc('-3 + 5')).toBe(2);
    expect(calc('10 / 4')).toBe(2.5);
    expect(calc('7 × 6')).toBe(42);
  });
  it('knows percent and remainder apart', () => {
    expect(calc('50%')).toBe(0.5);
    expect(calc('200 * 15%')).toBe(30);
    expect(calc('10 % 3')).toBe(1);
  });
  it('has a few functions and constants', () => {
    expect(calc('sqrt(81)')).toBe(9);
    expect(calc('abs(-4)')).toBe(4);
    expect(calc('round(2.6)')).toBe(3);
    expect(calc('pi')).toBeCloseTo(Math.PI);
    expect(calc('2 * pi')).toBeCloseTo(2 * Math.PI);
  });
  it('accepts a comma as the decimal mark', () => {
    expect(calc('1,5 + 1')).toBe(2.5);
  });
  it('says nothing for text that is not a calculation', () => {
    expect(calc('7')).toBeNull();
    expect(calc('')).toBeNull();
    expect(calc('hello')).toBeNull();
    expect(calc('2 +')).toBeNull();
    expect(calc('1 / 0')).toBeNull();
    expect(calc('sqrt(')).toBeNull();
    expect(calc('1.2.3 + 1')).toBeNull();
  });
  it('never runs code', () => {
    expect(calc('alert(1)')).toBeNull();
    expect(calc('constructor')).toBeNull();
    expect(calc('1; 2')).toBeNull();
    expect(calc('process.exit()')).toBeNull();
  });
});

describe('lastTwo', () => {
  it('keeps the end of a long folder path', () => {
    expect(lastTwo('C:\\Users\\a\\Docs\\Reports')).toBe('…\\Docs\\Reports');
    expect(lastTwo('/home/a/docs/reports')).toBe('…/docs/reports');
  });
  it('leaves a short path alone', () => {
    expect(lastTwo('C:\\Docs')).toBe('C:\\Docs');
  });
});

describe('showNumber', () => {
  it('hides float dust', () => {
    expect(showNumber(0.1 + 0.2)).toBe('0.3');
  });
  it('keeps whole numbers whole', () => {
    expect(showNumber(1234567)).toBe('1234567');
  });
  it('uses exponent form only for extremes', () => {
    expect(showNumber(1e20)).toMatch(/e/);
    expect(showNumber(0.00042)).toBe('0.00042');
  });
});
