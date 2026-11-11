import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import RecipeGripForm from './RecipeGripForm';

afterEach(() => {
  clearMocks();
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

const grip = () => useRecipeEditorStore.getState().editing?.draft.layers.find(l => l.id === 'l-2');

describe('a grip change', () => {
  it('offers the newer revision and takes it, keeping where it sits', async () => {
    await openRecipe('scythe-charge-held');
    const before = grip()!;
    render(<RecipeGripForm id="l-2" />);
    const group = await screen.findByRole('group', { name: 'Grip Scythe, staff grip r1' });
    expect(group).toHaveTextContent('Scythe, staff grip has r2');
    await userEvent.click(within(group).getByRole('button', { name: 'Update to r2' }));
    await waitFor(() => expect(grip()?.from?.revision).toBe(2));
    const r2 = await commands.gripLibraryGet('scythe-staff-grip', 2);
    expect(grip()?.pose).toEqual(r2.grip.pose);
    expect(grip()?.settings).toEqual(before.settings);
    expect(screen.queryByText(/has r2/)).toBeNull();
  });

  it('switches the hands from its own copy and holds a blank', async () => {
    await openRecipe('scythe-charge-held');
    render(<RecipeGripForm id="l-2" />);
    const group = await screen.findByRole('group', { name: /^Grip / });
    expect(within(group).getByRole('button', { name: 'Both' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    await userEvent.click(within(group).getByRole('button', { name: 'Left' }));
    expect(grip()?.bone).toBe('leftHand');
    expect(Object.keys(grip()?.pose ?? {}).every(b => b.startsWith('left'))).toBe(true);
    const eases = within(group).getByLabelText('Eases out');
    await userEvent.clear(eases);
    await userEvent.type(eases, '9{Enter}');
    expect(grip()?.settings.easeOut).toBe(9);
  });

  it('says when its library grip is gone', async () => {
    await openRecipe('scythe-charge-held');
    await commands.gripLibraryRemove('scythe-staff-grip', null);
    render(<RecipeGripForm id="l-2" />);
    expect(await screen.findByText('Its library grip is gone')).toBeVisible();
  });
});
