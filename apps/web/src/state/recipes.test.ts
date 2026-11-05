import { describe, expect, it } from 'vitest';
import type { RecipeSummary } from '@/ipc/client';
import { recipeLine } from './recipes';

const summary = (over: Partial<RecipeSummary> = {}): RecipeSummary => ({
  id: 'reach',
  file: 'recipes/reach.recipe.json',
  name: 'Reach',
  base: 'Standing Idle',
  frames: 120,
  mixamo: true,
  layers: 2,
  blanks: 1,
  open: 1,
  edited: '2026-11-05T10:00:00.000Z',
  problem: null,
  ...over,
});

describe('recipe lines', () => {
  it('names the base clip, frames, changes and open blanks', () => {
    expect(recipeLine(summary())).toBe('Standing Idle · 120 f · 2 changes · 1 open blank');
  });

  it('leaves out the blanks when none are open', () => {
    expect(recipeLine(summary({ layers: 1, open: 0 }))).toBe('Standing Idle · 120 f · 1 change');
  });
});
