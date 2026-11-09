import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import TrimForm from './TrimForm';

afterEach(clearMocks);

const clip = (i: number) => useRecipeEditorStore.getState().editing!.draft.clips[i];

describe('the recipe trim form', () => {
  it('shows what the trim keeps and changes its ends', async () => {
    await openRecipe('slash-into-block');
    render(<TrimForm />);
    expect(screen.getByText(/^Kept 40 of/)).toBeInTheDocument();
    await userEvent.clear(screen.getByLabelText('Out'));
    await userEvent.type(screen.getByLabelText('Out'), '40{Enter}');
    expect(clip(0)?.trim).toEqual([4, 40]);
  });

  it('notes the time edits a trim leaves out, and keeps every frame again', async () => {
    await openRecipe('slash-into-block');
    useRecipeEditorStore.getState().pick({ what: 'trim', id: '0' });
    render(<TrimForm />);
    await userEvent.clear(screen.getByLabelText('Out'));
    await userEvent.type(screen.getByLabelText('Out'), '20{Enter}');
    expect(screen.getByText(/^Outside the trim: Retime 1.6x/)).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Keep every frame' }));
    expect(clip(0)?.trim).toBeUndefined();
  });

  it('switches to another clip', async () => {
    await openRecipe('slash-into-block');
    render(<TrimForm />);
    await userEvent.selectOptions(screen.getByLabelText('Clip'), '1');
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'trim', id: '1' });
  });
});
