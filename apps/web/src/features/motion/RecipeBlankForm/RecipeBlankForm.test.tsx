import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeBlankForm from './RecipeBlankForm';

afterEach(clearMocks);

const blank = (id: string) =>
  useRecipeEditorStore.getState().editing?.draft.blanks.find(b => b.id === id);

describe('the recipe blank form', () => {
  it('picks a prop from the project assets', async () => {
    await openRecipe();
    render(<RecipeBlankForm id="lantern" />);
    const picker = screen.getByLabelText('Prop') as HTMLSelectElement;
    const other = [...picker.options].map(o => o.value).find(v => v && v !== picker.value)!;
    await userEvent.selectOptions(picker, other);
    expect(blank('lantern')?.prop).toBe(other);
  });

  it('turns a prop blank into a point and sets it in metres', async () => {
    await openRecipe();
    render(<RecipeBlankForm id="lantern" />);
    await userEvent.click(screen.getByRole('button', { name: 'Point' }));
    expect(blank('lantern')).toMatchObject({ kind: 'point', prop: null, point: null });
    await userEvent.clear(screen.getByLabelText('Y'));
    await userEvent.type(screen.getByLabelText('Y'), '1.2{Enter}');
    expect(blank('lantern')?.point).toEqual([0, 1.2, 0]);
  });

  it('removes the blank and leaves what aimed at it without a target', async () => {
    await openRecipe();
    render(<RecipeBlankForm id="lantern" />);
    await userEvent.click(screen.getByRole('button', { name: 'Remove blank' }));
    const draft = useRecipeEditorStore.getState().editing?.draft;
    expect(draft?.blanks.map(b => b.id)).toEqual(['shelf']);
    expect(draft?.layers[0]?.settings.target).toBeNull();
  });
});
