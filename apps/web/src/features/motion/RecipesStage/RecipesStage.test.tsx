import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { useRecipesStore } from '@/state/use-recipes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import RecipesStage from './RecipesStage';

afterEach(() => {
  clearMocks();
  useRecipesStore.setState(useRecipesStore.getInitialState(), true);
  useRecipeEditorStore.setState(useRecipeEditorStore.getInitialState(), true);
});

describe('the recipes stage', () => {
  it('says there are no recipes yet and opens the base clip picker', async () => {
    await openMockLibrary();
    render(<RecipesStage />);
    expect(await screen.findByText('No recipes yet')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'New recipe' }));
    expect(useRecipesStore.getState().picking).toBe('frostveil');
  });

  it('lists the project recipes with a Mixamo mark and Apply waiting for baking', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    expect(reach).toHaveTextContent('3 changes');
    expect(reach).not.toHaveTextContent('open blank');
    expect(reach).toHaveTextContent('Mixamo');
    expect(within(reach).getByRole('button', { name: /Apply/ })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    expect(screen.getByRole('button', { name: 'New recipe' })).toBeInTheDocument();
  });

  it('says when a prop blank reads open because its asset is gone', async () => {
    await openMockLibrary('demo');
    const warning =
      'The blank lantern names lantern, which is no longer an asset of Frostveil, so it reads as open.';
    vi.spyOn(commands, 'recipeList').mockResolvedValueOnce([
      {
        id: 'reach',
        file: 'recipes/reach.recipe.json',
        name: 'Reach for the lantern',
        base: 'Standing Idle',
        frames: 120,
        mixamo: false,
        layers: 1,
        blanks: 1,
        open: 1,
        edited: '2026-11-05T10:00:00.000Z',
        warnings: [warning],
        problem: null,
      },
    ]);
    render(<RecipesStage />);
    expect(await screen.findByText(warning)).toBeInTheDocument();
  });

  it('reads a gone prop as an open blank with its warning, from the demo', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const look = within(list).getByText('Look around').closest('li')!;
    expect(look).toHaveTextContent('1 open blank');
    expect(look).toHaveTextContent(/The blank torch names old-torch, which is no longer an asset/);
  });

  it('opens a recipe in the editor and goes back to the list', async () => {
    await openMockLibrary('demo');
    render(<RecipesStage />);
    const list = await screen.findByRole('list', { name: 'Project recipes' });
    const reach = within(list).getByText('Reach for the lantern').closest('li')!;
    await userEvent.click(within(reach).getByRole('button', { name: 'Edit' }));
    expect(
      await screen.findByRole('region', { name: 'Recipe Reach for the lantern' })
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    expect(await screen.findByRole('list', { name: 'Project recipes' })).toBeVisible();
  });
});
