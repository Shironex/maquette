import { describe, expect, it } from 'vitest';
import { settleOpen } from './agents';

describe('which approval cards are open', () => {
  it('opens the first of several', () => {
    expect(settleOpen({}, ['a', 'b', 'c'])).toEqual({ a: true, b: false, c: false });
  });

  it('keeps a later arrival folded while one is open', () => {
    expect(settleOpen({ a: true }, ['a', 'b'])).toEqual({ a: true, b: false });
  });

  it('does not open the next one when the open one is answered', () => {
    expect(settleOpen({ a: true, b: false }, ['b'])).toEqual({ b: false });
  });

  it('opens a new arrival when every other card is folded', () => {
    expect(settleOpen({ a: false }, ['a', 'b'])).toEqual({ a: false, b: true });
  });

  it('answers the same object when nothing changed', () => {
    const open = { a: true, b: false };
    expect(settleOpen(open, ['a', 'b'])).toBe(open);
  });
});
