import { describe, expect, it } from 'vitest';
import { fromLabel, usageLine } from './logic';

describe('translator text', () => {
  it('writes how much of the month is used', () => {
    expect(usageLine(12340, 500000)).toBe('12,340 of 500,000 characters used this month.');
    expect(usageLine(0, 500000)).toBe('0 of 500,000 characters used this month.');
  });
  it('shows the detected language in capitals, or nothing', () => {
    expect(fromLabel('pt')).toBe('PT');
    expect(fromLabel('')).toBe('');
  });
});
