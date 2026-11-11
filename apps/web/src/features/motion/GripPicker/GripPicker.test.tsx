import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { useRecipeEditorStore } from '@/state/use-recipe-editor-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import GripPicker from './GripPicker';

afterEach(() => {
  clearMocks();
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

async function picking(recipe?: string) {
  await openRecipe(recipe);
  act(() => useRecipeEditorStore.getState().pick({ what: 'grips', id: '' }));
  render(<GripPicker />);
  return screen.findByRole('list', { name: 'Grips to add' });
}

const added = () => useRecipeEditorStore.getState().editing?.draft.layers.at(-1);

describe('Add a grip', () => {
  it('adds the picked grip as a copy with a link back, over the frames asked for', async () => {
    const grid = await picking();
    await userEvent.click(within(grid).getByRole('button', { name: 'Dagger, reverse' }));
    const aside = screen.getByRole('complementary', { name: 'Add Dagger, reverse' });
    expect(within(aside).getByRole('button', { name: 'Right' })).toHaveAttribute(
      'aria-pressed',
      'true'
    );
    expect(aside).toHaveTextContent('It poses 15 finger bones');
    const to = within(aside).getByLabelText('To');
    await userEvent.clear(to);
    await userEvent.type(to, '30{Enter}');
    await userEvent.click(within(aside).getByRole('button', { name: 'Left' }));
    await userEvent.click(within(aside).getByRole('button', { name: 'Add grip to recipe' }));
    const layer = added();
    expect(layer).toMatchObject({
      kind: 'grip',
      bone: 'leftHand',
      mode: 'replace',
      from: { entry: 'dagger-reverse', revision: 1 },
      settings: { start: 0, easeIn: 4, easeOut: 4, target: null },
    });
    expect(Object.keys(layer?.pose ?? {}).every(b => b.startsWith('left'))).toBe(true);
    expect(layer?.settings.end).toBeGreaterThan(0);
    expect(layer?.settings.end).toBeLessThan(1);
    expect(useRecipeEditorStore.getState().picked).toEqual({ what: 'layer', id: layer?.id });
  });

  it('filters by weapon and keeps Add off while the range runs backwards', async () => {
    const grid = await picking();
    await userEvent.click(screen.getByRole('button', { name: 'Polearms' }));
    expect(within(grid).getAllByRole('button')).toHaveLength(1);
    await userEvent.click(within(grid).getByRole('button', { name: 'Scythe, staff grip' }));
    const aside = screen.getByRole('complementary', { name: 'Add Scythe, staff grip' });
    const from = within(aside).getByLabelText('From');
    await userEvent.clear(from);
    await userEvent.type(from, '9999{Enter}');
    expect(within(aside).getByRole('button', { name: 'Add grip to recipe' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('says when the recipe holds an older revision of the picked grip', async () => {
    const grid = await picking('scythe-charge-held');
    await userEvent.click(within(grid).getByRole('button', { name: 'Scythe, staff grip' }));
    const aside = screen.getByRole('complementary', { name: 'Add Scythe, staff grip' });
    expect(aside).toHaveTextContent('Scythe, staff grip has r2');
    expect(aside).toHaveTextContent('This recipe holds r1 in l-2. Update it there to take r2');
  });
});
