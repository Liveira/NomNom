import { describe, expect, it } from 'vitest';
import { load, percentOf, pushSample, rateText, sparkPoints, uptimeText } from './logic';

describe('monitor helpers', () => {
  it('keeps only the newest readings', () => {
    let h: number[] = [];
    for (let i = 0; i < 70; i++) h = pushSample(h, i);
    expect(h).toHaveLength(60);
    expect(h[0]).toBe(10);
    expect(h[59]).toBe(69);
    expect(pushSample([], Number.NaN)).toEqual([0]);
  });

  it('draws a line from the left to the right edge', () => {
    expect(sparkPoints([5], 100, 20)).toBe('');
    const pts = sparkPoints([0, 100], 100, 20, 100).split(' ');
    expect(pts).toEqual(['0.0,20.0', '100.0,0.0']);
    // The top is a fixed 100, so a quiet reading stays low on the chart.
    expect(sparkPoints([10, 10], 10, 10, 100)).toBe('0.0,9.0 10.0,9.0');
  });

  it('writes how long the computer has been on', () => {
    expect(uptimeText(8 * 60)).toBe('8 min');
    expect(uptimeText(2 * 3600 + 15 * 60)).toBe('2 h 15 min');
    expect(uptimeText(3 * 86_400 + 4 * 3600)).toBe('3 days 4 h');
    expect(uptimeText(86_400 + 3600)).toBe('1 day 1 h');
  });

  it('writes speeds', () => {
    expect(rateText(500)).toBe('500 B/s');
    expect(rateText(1536)).toBe('1.5 KB/s');
    expect(rateText(5 * 1024 * 1024)).toBe('5.0 MB/s');
  });

  it('tells how busy something is', () => {
    expect(load(10)).toBe('calm');
    expect(load(75)).toBe('busy');
    expect(load(95)).toBe('high');
    expect(percentOf(50, 200)).toBe(25);
    expect(percentOf(5, 0)).toBe(0);
    expect(percentOf(500, 200)).toBe(100);
  });
});

import { batteryLine, useText } from './logic';

describe('hardware words', () => {
  it('says what the battery is doing', () => {
    expect(batteryLine({ plugged: true, charging: true, minutes_left: null })).toBe('Charging');
    expect(batteryLine({ plugged: true, charging: false, minutes_left: null })).toBe('Plugged in');
    expect(batteryLine({ plugged: false, charging: false, minutes_left: 200 })).toBe('On battery: 3 h 20 min left');
    expect(batteryLine({ plugged: false, charging: false, minutes_left: 25 })).toBe('On battery: 25 min left');
    expect(batteryLine({ plugged: false, charging: false, minutes_left: null })).toBe('On battery');
  });
  it('turns hours of use into days when it is a while', () => {
    expect(useText(30)).toBe('30 hours of use');
    expect(useText(1200)).toBe('50 days of use');
  });
});
