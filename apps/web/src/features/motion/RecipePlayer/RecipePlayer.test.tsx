import { fireEvent, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipePlayer from './RecipePlayer';

afterEach(clearMocks);

describe('the recipe player', () => {
  it('labels the preview as rough, and pauses and scrubs over the clip', async () => {
    await openRecipe();
    render(<RecipePlayer />);
    expect(screen.getByText('Rough preview. The bake solves it exactly.')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(screen.getByRole('button', { name: 'Play' })).toBeInTheDocument();
    const playhead = screen.getByRole('slider', { name: 'Playhead' });
    const frames = Number(playhead.getAttribute('max'));
    expect(frames).toBeGreaterThan(10);
    fireEvent.change(playhead, { target: { value: '10' } });
    expect(screen.getByText(new RegExp(`^10 / ${frames} f · `))).toBeInTheDocument();
  });

  it("runs the playhead over the joined recipe's frames, not its base clip's", async () => {
    const store = await openRecipe('slash-into-block');
    render(<RecipePlayer />);
    const playhead = screen.getByRole('slider', { name: 'Playhead' });
    expect(Number(playhead.getAttribute('max'))).not.toBe(store.editing?.doc.recipe.base.frames);
    expect(screen.getByRole('region', { name: 'Timeline' })).toBeVisible();
  });
});
