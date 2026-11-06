import { fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { frameOf, shareOf } from '@/state/recipe-draft';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import ChangeBar from './ChangeBar';
import { moved } from './ChangeBar.hooks';

afterEach(clearMocks);

describe('a change bar', () => {
  it('moves a bar keeping its length and keeps its ends apart', () => {
    expect(moved('move', 10, 20, 50, 40)).toEqual([30, 40]);
    expect(moved('start', 10, 20, 15, 40)).toEqual([19, 20]);
    expect(moved('end', 10, 20, -15, 40)).toEqual([10, 11]);
  });

  it('moves the change a frame with the arrow keys, through the shares', async () => {
    const store = await openRecipe();
    const layer = store.editing!.draft.layers[0]!;
    const frames = store.editing!.doc.recipe.base.frames;
    const onPick = vi.fn();
    render(
      <ChangeBar
        layer={layer}
        frames={frames}
        toFrame={s => frameOf(s, frames)}
        toShare={f => shareOf(f, frames)}
        picked={false}
        onPick={onPick}
      />
    );
    const start = frameOf(layer.settings.start ?? 0, frames);
    fireEvent.keyDown(screen.getByRole('slider', { name: `Starts at frame ${start}` }), {
      key: 'ArrowRight',
    });
    const after = useRecipeEditorStore.getState().editing!.draft.layers[0]!.settings.start ?? 0;
    expect(frameOf(after, frames)).toBe(start + 1);
    fireEvent.click(screen.getByRole('button', { name: /^Reach l-1/ }));
    expect(onPick).toHaveBeenCalled();
  });
});
