import { describe, expect, it } from 'vitest';
import type { RecipeDrafts, RecipeLayer } from '@/ipc/client';
import { draftedKey, draftedLayer, draftsBanner, keysDrafted } from './recipe-drafts';

const NOW = Date.parse('2026-11-11T10:00:00Z');

const drafts = (over: Partial<RecipeDrafts> = {}): RecipeDrafts => ({
  by: 'Motion agent',
  at: '2026-11-11T09:56:00Z',
  writes: 2,
  made: false,
  layers: ['l-1'],
  keys: [
    { layer: 'l-1', clip: 0, frame: 20 },
    { layer: 'l-1', clip: 1, frame: 10 },
  ],
  clips: false,
  time: false,
  blanks: false,
  mirror: false,
  ...over,
});

describe('draft marks in words', () => {
  it('names the agent, when, what it drafted and what a save does', () => {
    expect(draftsBanner(drafts(), NOW)).toEqual({
      title: 'Motion agent changed this recipe 4 min ago',
      detail:
        'Drafted: 2 keys and 1 change. They are in the recipe now and bake as they are. Saving keeps them as yours.',
    });
    const made = draftsBanner(drafts({ made: true, keys: [], layers: [], time: true }), NOW);
    expect(made.title).toBe('Motion agent made this recipe 4 min ago');
    expect(made.detail).toMatch(/^Drafted: the time edits\./);
  });

  it('finds the drafted keys and changes, and counts a change keys', () => {
    const d = drafts();
    expect(draftedKey(d, 'l-1', 1, 10)).toBe(true);
    expect(draftedKey(d, 'l-1', 0, 10)).toBe(false);
    expect(draftedLayer(d, 'l-2')).toBe(false);
    const layer = {
      id: 'l-1',
      keys: [0, 6, 20].map(frame => ({ clip: 0, frame, pose: {} })),
    } as unknown as RecipeLayer;
    expect(keysDrafted(d, layer)).toBe('1 of 3 keys drafted');
    expect(keysDrafted(null, layer)).toBeNull();
  });
});
