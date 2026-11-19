import { describe, expect, it } from 'vitest';
import { arrowParts, blockSize, boxOf, clampBox, isReal, nextStep, type Shape } from './draw';

describe('boxOf', () => {
  it('is the same whichever way it was dragged', () => {
    expect(boxOf({ x: 10, y: 40 }, { x: 4, y: 12 })).toEqual({ x: 4, y: 12, w: 6, h: 28 });
    expect(boxOf({ x: 4, y: 12 }, { x: 10, y: 40 })).toEqual({ x: 4, y: 12, w: 6, h: 28 });
  });
});

describe('clampBox', () => {
  it('keeps a box inside the picture', () => {
    expect(clampBox({ x: -5, y: -5, w: 30, h: 30 }, 100, 100)).toEqual({ x: 0, y: 0, w: 25, h: 25 });
    expect(clampBox({ x: 90, y: 90, w: 30, h: 30 }, 100, 100)).toEqual({ x: 90, y: 90, w: 10, h: 10 });
    expect(clampBox({ x: 200, y: 200, w: 10, h: 10 }, 100, 100)).toEqual({ x: 100, y: 100, w: 0, h: 0 });
  });
});

describe('arrowParts', () => {
  it('puts the head at the end and the line short of it', () => {
    const a = arrowParts({ x: 0, y: 0 }, { x: 100, y: 0 }, 4)!;
    expect(a.tip).toEqual({ x: 100, y: 0 });
    expect(a.base.x).toBeLessThan(100);
    expect(a.base.y).toBeCloseTo(0);
    // The two corners are on either side of the line, the same distance from it.
    expect(a.left.y).toBeCloseTo(-a.right.y);
    expect(a.left.y).not.toBeCloseTo(0);
  });
  it('draws nothing for a stray click', () => {
    expect(arrowParts({ x: 5, y: 5 }, { x: 6, y: 5 }, 4)).toBeNull();
  });
  it('keeps the head smaller than a short arrow', () => {
    const a = arrowParts({ x: 0, y: 0 }, { x: 10, y: 0 }, 7)!;
    expect(a.base.x).toBeGreaterThan(0);
  });
});

describe('blur and steps', () => {
  it('uses blocks too big to read through', () => {
    expect(blockSize({ x: 0, y: 0, w: 400, h: 80 })).toBe(10);
    expect(blockSize({ x: 0, y: 0, w: 20, h: 20 })).toBe(8);
  });
  it('numbers steps one after the other', () => {
    const shapes: Shape[] = [
      { kind: 'rect', color: '#fff', width: 2, box: { x: 0, y: 0, w: 5, h: 5 } },
      { kind: 'step', color: '#fff', width: 2, at: { x: 1, y: 1 }, n: 1 },
      { kind: 'step', color: '#fff', width: 2, at: { x: 2, y: 2 }, n: 2 },
    ];
    expect(nextStep(shapes)).toBe(3);
    expect(nextStep([])).toBe(1);
  });
  it('tells a real box from a click', () => {
    expect(isReal({ x: 0, y: 0, w: 2, h: 40 })).toBe(false);
    expect(isReal({ x: 0, y: 0, w: 30, h: 40 })).toBe(true);
  });
});
