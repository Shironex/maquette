import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { useRecipeSaveStore } from '@/state/use-recipe-save-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import SaveRecipeSheet from './SaveRecipeSheet';

afterEach(() => {
  clearMocks();
  useRecipeSaveStore.setState(useRecipeSaveStore.getInitialState(), true);
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
});

/** The demo workspace with the sheet open on `recipe`, planned. */
async function openOn(recipe = 'reach-for-the-lantern', entry?: string | null) {
  await openMockLibrary('demo');
  render(<SaveRecipeSheet />);
  await act(() => useRecipeSaveStore.getState().open('frostveil', recipe, entry));
  return screen.getByRole('dialog', { name: 'Save to library' });
}

describe('the save to library sheet', () => {
  it('lists what changes against the newest revision of the library recipe it came from', async () => {
    const sheet = await openOn();
    expect(
      within(sheet).getByRole('button', { name: 'New revision of Reach for the lantern' })
    ).toHaveAttribute('aria-pressed', 'true');
    expect(within(sheet).getByText('What changes against r3')).toBeVisible();
    expect(within(sheet).getByRole('list', { name: 'Changes' })).toHaveTextContent(
      'Removes the nudge on spine (l-4).'
    );
    expect(within(sheet).getByRole('button', { name: 'Save as r4' })).toBeEnabled();
  });

  it('plans a new library recipe when that is the target', async () => {
    const sheet = await openOn();
    await userEvent.click(within(sheet).getByRole('button', { name: 'New library recipe' }));
    expect(await within(sheet).findByText(/^A new library recipe on /)).toBeVisible();
    expect(within(sheet).getByRole('button', { name: 'Save as a new recipe' })).toBeEnabled();
  });

  it('offers only a new library recipe for a recipe that came from none', async () => {
    const sheet = await openOn('look-around');
    expect(within(sheet).queryByRole('button', { name: /^New revision of/ })).toBeNull();
    expect(within(sheet).getByText('What it holds')).toBeVisible();
  });

  it('saves, closes, says where it went, and then has nothing to save', async () => {
    const sheet = await openOn();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Save as r4' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().notice?.title).toBe(
      'Saved Reach for the lantern as r4 of Reach for the lantern'
    );
    const again = await (async () => {
      await act(() => useRecipeSaveStore.getState().open('frostveil', 'reach-for-the-lantern'));
      return screen.getByRole('dialog', { name: 'Save to library' });
    })();
    expect(within(again).getByText('Nothing changed since r4.')).toBeVisible();
    expect(within(again).getByRole('button', { name: 'Save as r5' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('reads the library recipe again when it moved, and waits for a look', async () => {
    const sheet = await openOn();
    const planned = vi.spyOn(commands, 'recipeLibraryPlanSave');
    vi.spyOn(commands, 'recipeLibrarySave').mockRejectedValueOnce({
      code: 'moved',
      message: 'The library recipe reach-for-the-lantern changed since it was read.',
    });
    await userEvent.click(within(sheet).getByRole('button', { name: 'Save as r4' }));
    expect(
      await within(sheet).findByText(
        'The library recipe changed since this sheet opened. It was read again; look before saving.'
      )
    ).toBeVisible();
    expect(planned).toHaveBeenCalledWith(
      'frostveil',
      'reach-for-the-lantern',
      'reach-for-the-lantern'
    );
    expect(screen.getByRole('dialog', { name: 'Save to library' })).toBeVisible();
  });

  it('opens preset to a new library recipe from an agent proposal', async () => {
    const sheet = await openOn('reach-for-the-lantern', null);
    expect(within(sheet).getByRole('button', { name: 'Save as a new recipe' })).toBeEnabled();
  });
});
