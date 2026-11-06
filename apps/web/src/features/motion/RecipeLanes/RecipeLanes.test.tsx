import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeLanes from './RecipeLanes';

afterEach(clearMocks);

describe('the recipe lanes', () => {
  it('lays the clips, trims, retime, freeze and mirror over the bake frames', async () => {
    await openRecipe('slash-into-block');
    render(<RecipeLanes frame={0} playing={false} onToggle={vi.fn()} onScrub={vi.fn()} />);
    expect(
      screen.getByRole('button', { name: /^Clip 2, Sword And Shield Block 2 to 28, Hash differs/ })
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Retime 1.6x' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Freeze 12 f' })).toBeVisible();
    expect(screen.getByRole('button', { name: 'Mirror off' })).toBeVisible();
    expect(screen.getByText(/^0 \/ \d+ f · 0\.00 \/ /)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Freeze 12 f' }));
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'time', id: '1' });
  });

  it('scrubs and plays from its own controls', async () => {
    await openRecipe();
    const onToggle = vi.fn();
    render(<RecipeLanes frame={0} playing onToggle={onToggle} onScrub={vi.fn()} />);
    await userEvent.click(screen.getByRole('button', { name: 'Pause' }));
    expect(onToggle).toHaveBeenCalled();
    expect(screen.getByRole('button', { name: /^Reach l-1, frames/ })).toBeVisible();
  });
});
