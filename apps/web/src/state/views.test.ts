import { describe, expect, it } from 'vitest';
import {
  LIBRARY_VIEWS,
  hashForView,
  plannedViewsFor,
  showsView,
  toolViewsFor,
  viewFromHash,
} from './views';

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
    expect(viewFromHash(hashForView('library'))).toBe('library');
    expect(viewFromHash(hashForView('animations'))).toBe('animations');
    expect(viewFromHash(hashForView('avatar'))).toBe('avatar');
  });

  it('keeps Models and Animations under Libraries, out of the tools', () => {
    expect(LIBRARY_VIEWS.map(v => [v.id, v.label])).toEqual([
      ['library', 'Models'],
      ['animations', 'Animations'],
    ]);
    expect(toolViewsFor('game').map(v => v.id)).not.toContain('library');
    expect(showsView('game', 'animations')).toBe(true);
  });

  it('hides Page and Timeline for a game and keeps Motion', () => {
    const game = toolViewsFor('game').map(v => v.id);
    expect(game).not.toContain('page');
    expect(game).not.toContain('timeline');
    expect(game).toContain('motion');
    expect(game).toContain('avatar');
    expect(toolViewsFor('website').map(v => v.id)).toContain('page');
    expect(showsView('game', 'page')).toBe(false);
    expect(showsView('website', 'page')).toBe(true);
  });

  it('lists Game preview as planned for a game only', () => {
    expect(plannedViewsFor('game')).toEqual([
      expect.objectContaining({ label: 'Game preview', tag: 'later' }),
    ]);
    expect(plannedViewsFor('website')).toEqual([]);
  });
});
