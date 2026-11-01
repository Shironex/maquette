import { describe, expect, it } from 'vitest';
import type { Asset } from '@/ipc/client';
import { boardPrimary } from './board';

const card = { versions: [] } as unknown as Asset;
const model = { versions: [{ version: 1 }] } as unknown as Asset;
const state = { open: true, running: false, tips: true };

describe('the Board primary', () => {
  it('has none with no project', () => {
    expect(boardPrimary([], { ...state, open: false })).toBeNull();
  });

  it('leaves an empty Board to its first run card, then to Add once the tips are hidden', () => {
    expect(boardPrimary([], state)).toBeNull();
    expect(boardPrimary([], { ...state, tips: false })).toBe('add');
  });

  it('is Add with cards but no model, never Run pipeline', () => {
    expect(boardPrimary([card], state)).toBe('add');
  });

  it('is Run pipeline with a model, and nothing while a run goes', () => {
    expect(boardPrimary([card, model], state)).toBe('run');
    expect(boardPrimary([model], { ...state, running: true })).toBeNull();
  });
});
