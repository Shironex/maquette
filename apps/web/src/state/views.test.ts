import { describe, expect, it } from 'vitest';
import { hashForView, viewFromHash } from './views';

describe('views', () => {
  it('reads the view from the hash', () => {
    expect(viewFromHash('#/settings')).toBe('settings');
    expect(viewFromHash('#settings')).toBe('settings');
    expect(viewFromHash('#/inbox/model-3')).toBe('inbox');
  });

  it('falls back to the Board for anything unknown', () => {
    expect(viewFromHash('')).toBe('board');
    expect(viewFromHash('#/nowhere')).toBe('board');
  });

  it('round trips', () => {
    expect(viewFromHash(hashForView('pipeline'))).toBe('pipeline');
    expect(viewFromHash(hashForView('page'))).toBe('page');
    expect(viewFromHash(hashForView('timeline'))).toBe('timeline');
  });
});
