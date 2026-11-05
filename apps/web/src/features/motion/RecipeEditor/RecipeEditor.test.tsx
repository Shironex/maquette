import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands, type RecipeLayer } from '@/ipc/client';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeEditor from './RecipeEditor';

afterEach(clearMocks);

const weigh = (weight: number) => (l: RecipeLayer[]) =>
  l.map(x => (x.id === 'l-1' ? { ...x, settings: { ...x.settings, weight } } : x));
const edit = (weight: number) =>
  act(() => useRecipeEditorStore.getState().edit(d => ({ ...d, layers: weigh(weight)(d.layers) })));

describe('the recipe editor', () => {
  it('saves only with unsaved edits, then says saved', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('aria-disabled', 'true');
    edit(0.5);
    expect(screen.getByText(/unsaved edits$/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    expect(await screen.findByText(/· saved$/)).toBeVisible();
  });

  it('asks before Back drops unsaved edits, and stays when asked to', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    edit(0.5);
    await userEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    const dialog = screen.getByRole('dialog', { name: 'Save the recipe first?' });
    expect(dialog).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Stay' }));
    expect(useRecipeEditorStore.getState().editing?.draft.layers[0]?.settings.weight).toBe(0.5);
    await userEvent.click(screen.getByRole('button', { name: 'Recipes' }));
    await userEvent.click(screen.getByRole('button', { name: 'Leave without saving' }));
    expect(useRecipeEditorStore.getState().editing).toBeNull();
  });

  it('asks Keep mine or Use theirs when an agent changed what the person edited', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    edit(0.5);
    const doc = await commands.recipeGet('frostveil', 'reach-for-the-lantern');
    await commands.recipeUpdate('frostveil', 'reach-for-the-lantern', {
      name: null,
      layers: weigh(0.9)(doc.recipe.layers),
      blanks: null,
    });
    await act(() => useRecipeEditorStore.getState().onChanged('frostveil'));
    expect(screen.getByRole('dialog', { name: 'The recipe changed outside' })).toHaveTextContent(
      'Both sides changed change l-1.'
    );
    await userEvent.keyboard('{Escape}');
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('aria-disabled', 'true');
    await userEvent.click(screen.getByRole('button', { name: 'Answer clash' }));
    await userEvent.click(screen.getByRole('button', { name: 'Keep mine' }));
    expect(useRecipeEditorStore.getState().editing?.draft.layers[0]?.settings.weight).toBe(0.5);
  });

  it('says what to fix before Save', async () => {
    await openRecipe();
    render(<RecipeEditor />);
    act(() =>
      useRecipeEditorStore.getState().edit(d => ({
        ...d,
        layers: d.layers.map(l =>
          l.id === 'l-1' ? { ...l, settings: { ...l.settings, target: null } } : l
        ),
      }))
    );
    expect(screen.getByRole('button', { name: 'Save' })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getByText('A reach needs a target: pick a blank, or add one.')).toBeVisible();
  });
});
