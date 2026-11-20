import { describe, expect, it } from 'vitest';
import { convert } from './units';

describe('convert', () => {
  it('converts lengths, masses and volumes', () => {
    expect(convert('10 km to mi')?.value).toBeCloseTo(6.21371, 4);
    expect(convert('1 mile in km')?.value).toBeCloseTo(1.609344, 5);
    expect(convert('5 kg to lb')?.value).toBeCloseTo(11.0231, 3);
    expect(convert('500 ml to cups')?.value).toBeCloseTo(2.1134, 3);
    expect(convert('6 ft to cm')?.value).toBeCloseTo(182.88, 2);
  });
  it('does temperatures properly', () => {
    expect(convert('72 f to c')?.value).toBeCloseTo(22.2222, 3);
    expect(convert('100 c in f')?.value).toBeCloseTo(212, 5);
    expect(convert('0 c to k')?.value).toBeCloseTo(273.15, 5);
    expect(convert('-40 f to c')?.value).toBeCloseTo(-40, 5);
  });
  it('knows data sizes, speeds and times', () => {
    expect(convert('1 gib to mb')?.value).toBeCloseTo(1073.741824, 5);
    expect(convert('100 km/h to mph')?.value).toBeCloseTo(62.1371, 3);
    expect(convert('2h to min')?.value ?? convert('2 h to min')?.value).toBeCloseTo(120, 5);
    expect(convert('90 min to h')?.value).toBeCloseTo(1.5, 5);
  });
  it('reads Portuguese and Spanish words and a decimal comma', () => {
    expect(convert('2,5 quilos para libras')?.value).toBeCloseTo(5.5116, 3);
    expect(convert('3 milhas em km')?.value).toBeCloseTo(4.828, 3);
    expect(convert('10 metros a pies')?.value).toBeCloseTo(32.8084, 3);
  });
  it('says nothing for what is not a conversion', () => {
    expect(convert('10 km to kg')).toBeNull();
    expect(convert('hello to world')).toBeNull();
    expect(convert('10 km')).toBeNull();
    expect(convert('2 + 3')).toBeNull();
    expect(convert('10 usd to eur')).toBeNull();
    expect(convert('')).toBeNull();
  });
  it('names the units it used', () => {
    const r = convert('10 km to mi')!;
    expect([r.from, r.to]).toEqual(['km', 'mi']);
    const t = convert('72 f to c')!;
    expect([t.from, t.to]).toEqual(['°F', '°C']);
  });
});
