import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeBaseEdits from './RecipeBaseEdits';

afterEach(clearMocks);

describe('the recipe base edits panel', () => {
  it('lists the clips, trim, each time edit and the mirror, and warns of a changed clip', async () => {
    await openRecipe('slash-into-block');
    render(<RecipeBaseEdits />);
    const rows = within(screen.getByRole('list', { name: 'Recipe base edits' })).getAllByRole(
      'button'
    );
    expect(rows.map(r => r.textContent)).toEqual([
      expect.stringMatching(/^3 clips, joined.*blends 6 and 8 f/),
      expect.stringMatching(/^Trim/),
      expect.stringMatching(/^Retime 1.6x/),
      expect.stringMatching(/^Freeze 12 f/),
      expect.stringMatching(/^MirrorOff/),
    ]);
    expect(screen.getByText('Sword And Shield Block changed in the library')).toBeInTheDocument();
  });

  it('opens one base edit at a time', async () => {
    await openRecipe('slash-into-block');
    render(<RecipeBaseEdits />);
    await userEvent.click(screen.getByRole('button', { name: /^Freeze 12 f/ }));
    expect(screen.getByRole('group', { name: 'Time edit 2 settings' })).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: /^Mirror/ }));
    expect(screen.queryByRole('group', { name: 'Time edit 2 settings' })).not.toBeInTheDocument();
    expect(screen.getByRole('group', { name: 'Mirror settings' })).toBeInTheDocument();
  });

  it('adds a retime from the Add menu and opens it, or a clip through the picker', async () => {
    await openRecipe('slash-into-block');
    render(<RecipeBaseEdits />);
    await userEvent.click(screen.getByRole('button', { name: 'Add' }));
    await userEvent.click(screen.getByRole('menuitem', { name: /Add a retime/ }));
    expect(useRecipeEditorStore.getState().editing?.draft.time).toHaveLength(3);
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'time', id: '2' });
    await userEvent.click(screen.getByRole('button', { name: 'Add' }));
    await userEvent.click(screen.getByRole('menuitem', { name: /Add a clip/ }));
    expect(useRecipesStore.getState()).toMatchObject({ joining: true });
  });
});
