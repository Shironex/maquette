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

  it('says the file keeps a gone prop, and Leave open empties it for good until kept again', async () => {
    await openRecipe('look-around');
    render(<RecipeBlankForm id="torch" />);
    expect(screen.getByText(/The file still names old-torch/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Leave open' }));
    expect(useRecipeEditorStore.getState().editing?.cleared).toEqual(['torch']);
    expect(screen.getByText(/Saving empties it for good/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Keep old-torch' }));
    expect(useRecipeEditorStore.getState().editing?.cleared).toEqual([]);
  });

  it("keeps picking from the recipe's own project after another project opens", async () => {
    await openRecipe();
    const { useAppStore } = await import('@/state/use-app-store');
    const kept = useRecipeEditorStore.getState().editing?.assets.length ?? 0;
    expect(kept).toBeGreaterThan(0);
    const workspace = useAppStore.getState().workspace!;
    useAppStore.setState({
      workspace: { ...workspace, project: { ...workspace.project!, id: 'other', assets: [] } },
    });
    render(<RecipeBlankForm id="lantern" />);
    const picker = screen.getByLabelText('Prop') as HTMLSelectElement;
    expect(picker.options).toHaveLength(kept + 1);
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
