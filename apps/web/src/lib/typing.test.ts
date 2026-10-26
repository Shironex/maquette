import { describe, expect, it } from 'vitest';
import { typing } from './typing';

describe('typing', () => {
  it('tells a text field from the rest of the page', () => {
    const input = document.createElement('input');
    const button = document.createElement('button');
    expect(typing(input)).toBe(true);
    expect(typing(button)).toBe(false);
    expect(typing(window)).toBe(false);
  });
});
