import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeCompareStore } from '@/state/use-recipe-compare-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import RecipeCompareSheet from './RecipeCompareSheet';

afterEach(() => {
  clearMocks();
  useRecipeCompareStore.setState(useRecipeCompareStore.getInitialState(), true);
});

async function openOn(recipe: string) {
  await openMockLibrary('demo');
  render(<RecipeCompareSheet />);
  await act(() => useRecipeCompareStore.getState().open('frostveil', recipe));
  return screen.getByRole('dialog', { name: 'What changed' });
}

describe('the What changed sheet', () => {
  it('lists what the newest revision changes and uses it as a new recipe', async () => {
    const sheet = await openOn('reach-for-the-lantern');
    expect(within(sheet).getByText('Reach for the lantern: r1 to r3')).toBeVisible();
    expect(within(sheet).getByRole('list', { name: 'Changes' }).children).toHaveLength(2);
    await userEvent.click(within(sheet).getByRole('button', { name: 'Use r3' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().notice?.title).toBe('Made Reach for the lantern (2) from r3');
    const ids = (await commands.recipeList('frostveil')).map(r => r.id);
    expect(ids).toContain('reach-for-the-lantern-2');
    expect(ids).toContain('reach-for-the-lantern');
  });

  it('says why a recipe cannot be compared', async () => {
    const sheet = await openOn('look-around');
    expect(
      await within(sheet).findByText('Look around came from no library recipe.')
    ).toBeVisible();
    expect(within(sheet).getByRole('button', { name: 'Use' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});
