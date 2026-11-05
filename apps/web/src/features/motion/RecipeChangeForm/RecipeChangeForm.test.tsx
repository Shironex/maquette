import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeChangeForm from './RecipeChangeForm';

afterEach(clearMocks);

const layer = (id: string) =>
  useRecipeEditorStore.getState().editing?.draft.layers.find(l => l.id === id);

describe('the recipe change form', () => {
  it('limits the bones to the kind and shows the target blanks', async () => {
    await openRecipe();
    render(<RecipeChangeForm id="l-1" />);
    const bones = [...(screen.getByLabelText('Bone') as HTMLSelectElement).options].map(
      o => o.value
    );
    expect(bones).toEqual(['leftHand', 'rightHand', 'leftFoot', 'rightFoot']);
    expect(screen.getByLabelText('Target')).toHaveValue('lantern');
    expect(screen.getByLabelText('Start frame')).toBeInTheDocument();
  });

  it('switches kind with the settings it reads, and says what is missing inline', async () => {
    await openRecipe();
    render(<RecipeChangeForm id="l-1" />);
    await userEvent.selectOptions(screen.getByLabelText('Target'), '');
    expect(screen.getByRole('alert')).toHaveTextContent('A reach needs a target');
    await userEvent.selectOptions(screen.getByLabelText('Kind'), 'nudge');
    expect(layer('l-1')).toMatchObject({ kind: 'nudge', bone: 'rightHand' });
    expect(screen.getByLabelText('Turn Y')).toBeInTheDocument();
    await userEvent.clear(screen.getByLabelText('Move Y'));
    await userEvent.type(screen.getByLabelText('Move Y'), '0.1{Enter}');
    expect(layer('l-1')?.settings.offset).toEqual([0, 0.1, 0]);
  });

  it('removes the change', async () => {
    await openRecipe();
    render(<RecipeChangeForm id="l-3" />);
    await userEvent.click(screen.getByRole('button', { name: 'Remove change' }));
    expect(layer('l-3')).toBeUndefined();
  });
});
