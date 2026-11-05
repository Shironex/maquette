import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeChanges from './RecipeChanges';

afterEach(clearMocks);

describe('the recipe changes', () => {
  it('lists the changes in order and opens one to edit', async () => {
    await openRecipe();
    render(<RecipeChanges />);
    const list = screen.getByRole('list', { name: 'Recipe changes' });
    const rows = within(list).getAllByRole('button', { expanded: false });
    expect(rows.map(r => r.textContent)).toEqual([
      expect.stringContaining('Reach · Right hand → lantern'),
      expect.stringContaining('Look at · Head → shelf'),
      expect.stringContaining('Wave · Left lower arm'),
    ]);
    await userEvent.click(rows[0]!);
    expect(screen.getByRole('group', { name: 'Change l-1 settings' })).toBeVisible();
  });

  it('adds a change of the kind picked and opens it', async () => {
    await openRecipe();
    render(<RecipeChanges />);
    await userEvent.selectOptions(screen.getByLabelText('Kind of change'), 'nudge');
    await userEvent.click(screen.getByRole('button', { name: 'Add change' }));
    const draft = useRecipeEditorStore.getState().editing?.draft;
    const added = draft?.layers.at(-1);
    expect(added).toMatchObject({ kind: 'nudge', bone: 'spine' });
    expect(added?.id).toMatch(/^l-[a-z0-9]{5}$/);
    expect(screen.getByRole('group', { name: `Change ${added?.id} settings` })).toBeVisible();
  });

  it('marks a change the core would refuse on its row', async () => {
    await openRecipe();
    useRecipeEditorStore.getState().edit(d => ({
      ...d,
      layers: d.layers.map(l =>
        l.id === 'l-1' ? { ...l, settings: { ...l.settings, target: null } } : l
      ),
    }));
    render(<RecipeChanges />);
    expect(screen.getByText('A reach needs a target: pick a blank, or add one.')).toBeVisible();
  });
});
