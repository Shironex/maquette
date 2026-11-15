import { render, screen, within } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import type { LibraryRecipeDoc, RecipeBase, RecipeFile } from '@/ipc/client';
import { timeline } from '@/state/recipe-timeline';
import RecipeHolds from './RecipeHolds';

const clip = (name: string, frames: number, over: Partial<RecipeBase> = {}): RecipeBase => ({
  name,
  frames,
  length: frames / 30,
  hash: name,
  mixamo: false,
  ...over,
});

const nudge = {
  id: 'l-1',
  kind: 'nudge',
  bone: 'spine',
  settings: { weight: 1, start: 0.5, end: 1, target: null, rotation: [8, 0, 0] },
} as unknown as RecipeFile['layers'][number];

function docOf(recipe: RecipeFile): LibraryRecipeDoc {
  return {
    entry: { id: 'r', uid: 'u', name: recipe.name, createdAt: '', updatedAt: '', revisions: [] },
    version: 'v',
    revision: 1,
    recipe,
    base: { status: 'found', warning: null },
    mixamo: false,
  } as unknown as LibraryRecipeDoc;
}

const recipe = (over: Partial<RecipeFile> = {}): RecipeFile => ({
  version: 2,
  name: 'Charge',
  revision: 1,
  created: '',
  edited: '',
  base: clip('Scythe Charge', 45),
  blanks: [],
  layers: [nudge],
  ...over,
});

describe('what a library revision holds', () => {
  it('names the joined clips in order and the length they run together', () => {
    const joined = recipe({
      joined: [
        clip('Great Sword Idle', 60, { blend: { frames: 6, curve: 'smooth', lineUp: 'hips' } }),
        clip('Scythe Swing', 30),
      ],
    });
    const frames = timeline(joined).frames;
    expect(frames).toBeGreaterThan(45);
    render(<RecipeHolds doc={docOf(joined)} />);
    expect(screen.getByText(`On Scythe Charge + 2 clips, ${frames} frames`)).toBeInTheDocument();
    const clips = within(screen.getByRole('list', { name: 'Clips' })).getAllByRole('listitem');
    expect(clips.map(c => c.textContent)).toEqual([
      '1. Scythe Charge',
      '2. Great Sword Idle',
      '3. Scythe Swing',
    ]);
    // The change's frames stand on the whole joined timeline, not on the base clip alone.
    const change = within(screen.getByRole('list', { name: 'Changes' })).getByRole('listitem');
    expect(change.textContent).not.toMatch(/to 45$/);
    expect(change.textContent).toMatch(new RegExp(`to ${frames}$`));
  });

  it('keeps a one clip recipe as it was, with no clip list', () => {
    render(<RecipeHolds doc={docOf(recipe())} />);
    expect(screen.getByText('On Scythe Charge, 45 frames')).toBeInTheDocument();
    expect(screen.queryByRole('list', { name: 'Clips' })).toBeNull();
    expect(screen.getByRole('list', { name: 'Changes' })).toHaveTextContent('frames 23 to 45');
  });
});
