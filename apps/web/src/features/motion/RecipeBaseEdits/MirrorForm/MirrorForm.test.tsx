import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import MirrorForm from './MirrorForm';

afterEach(clearMocks);

const draft = () => useRecipeEditorStore.getState().editing!.draft;

describe('the recipe mirror form', () => {
  it('turns the mirror on straight away when no change names a side', async () => {
    await openRecipe('slash-into-block');
    render(<MirrorForm />);
    await userEvent.click(screen.getByRole('button', { name: 'Left handed' }));
    expect(draft().mirror).toBe(true);
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('asks whether to swap sides when the changes name one, listing the swaps', async () => {
    await openRecipe();
    render(<MirrorForm />);
    const before = draft().layers.map(l => l.bone);
    await userEvent.click(screen.getByRole('button', { name: 'Left handed' }));
    const dialog = screen.getByRole('dialog');
    expect(dialog).toHaveTextContent(/to left/);
    await userEvent.click(screen.getByRole('button', { name: 'Swap sides' }));
    expect(draft().mirror).toBe(true);
    expect(draft().layers.map(l => l.bone)).not.toEqual(before);
  });
});
