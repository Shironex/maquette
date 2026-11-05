import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryRecipes from './LibraryRecipes';

afterEach(() => {
  clearMocks();
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
});

describe('the library recipes', () => {
  it('says there are none yet and how to get one', async () => {
    await openMockLibrary();
    render(<LibraryRecipes />);
    expect(await screen.findByText('No library recipes yet')).toBeVisible();
    expect(
      screen.getByText(
        "Save a recipe from the Motion view's Recipes tab, or import a .recipe.json."
      )
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Import recipe' })).toBeVisible();
  });

  it('lists each entry with its line and Mixamo mark, the first on show', async () => {
    await openMockLibrary('demo');
    render(<LibraryRecipes />);
    const list = await screen.findByRole('list', { name: 'Library recipes' });
    expect(screen.getByText('3 recipes')).toBeVisible();
    const reach = within(list).getByRole('button', { name: 'Reach for the lantern' });
    expect(reach).toHaveTextContent(' f · 4 changes · r3');
    const guard = within(list).getByRole('button', { name: 'Guard and look' });
    expect(guard).toHaveAttribute('aria-pressed', 'true');
    expect(guard).toHaveTextContent('Mixamo');
    expect(screen.getByRole('complementary', { name: 'Selected recipe' })).toHaveTextContent(
      'Guard and look'
    );
    await userEvent.click(reach);
    expect(reach).toHaveAttribute('aria-pressed', 'true');
  });

  it('shows why an entry could not be read', async () => {
    await openMockLibrary('demo');
    vi.spyOn(commands, 'recipeLibraryList').mockResolvedValueOnce([
      {
        id: 'broken',
        uid: '',
        name: 'broken',
        version: 'v',
        latest: 0,
        revisions: [],
        base: '',
        frames: 0,
        mixamo: false,
        layers: 0,
        blanks: 0,
        updatedAt: '',
        problem: 'entry.json is not valid JSON.',
      },
    ]);
    render(<LibraryRecipes />);
    expect(await screen.findAllByText('entry.json is not valid JSON.')).not.toHaveLength(0);
  });

  it('imports a recipe, selects it and says what to know', async () => {
    await openMockLibrary('demo');
    render(<LibraryRecipes />);
    await screen.findByRole('list', { name: 'Library recipes' });
    await userEvent.click(screen.getByRole('button', { name: 'Import recipe' }));
    const note = (await screen.findByText('Imported Sit and wave as r1')).closest('[role]')!;
    expect(note).toHaveTextContent(/hash differs/);
    expect(screen.getByRole('button', { name: 'Sit and wave' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByText('Imported Sit and wave as r1')).toBeNull();
  });
});
