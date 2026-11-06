import { act, render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useMotionStore } from '@/state/use-motion-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import UseRecipeSheet from './UseRecipeSheet';

afterEach(() => {
  clearMocks();
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
  useMotionStore.setState(useMotionStore.getInitialState(), true);
});

const target = { entry: 'reach-for-the-lantern', revision: 2, name: 'Reach for the lantern' };

describe('Use in project', () => {
  it('shows nothing until a revision is picked to use', async () => {
    await openMockLibrary('demo');
    render(<UseRecipeSheet />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('copies the revision into the open project, says what to know and opens Motion', async () => {
    await openMockLibrary('demo');
    useRecipeLibraryStore.getState().openUse(target);
    const used = vi.spyOn(commands, 'recipeLibraryUse');
    render(<UseRecipeSheet />);
    const sheet = screen.getByRole('dialog', { name: 'Use in project' });
    expect(within(sheet).getByRole('combobox', { name: 'Project' })).toHaveValue('frostveil');
    await userEvent.type(within(sheet).getByLabelText(/Recipe name/), 'Reach again');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Use in project' }));
    expect(used).toHaveBeenCalledWith({ ...target, project: 'frostveil', name: 'Reach again' });
    expect(await within(sheet).findByText('Made Reach again in Frostveil')).toBeVisible();
    await userEvent.click(within(sheet).getByRole('button', { name: 'Open in Motion' }));
    expect(useRecipeLibraryStore.getState().using).toBeNull();
    expect(useMotionStore.getState().tab).toBe('recipes');
    expect(useAppStore.getState().view).toBe('motion');
  });

  it('notes what the copy could not keep, and says why a use was refused', async () => {
    await openMockLibrary('demo');
    useRecipeLibraryStore.getState().openUse(target);
    render(<UseRecipeSheet />);
    await userEvent.click(screen.getByRole('button', { name: 'Use in project' }));
    expect(await screen.findByText(/has a recipe reach-for-the-lantern already/)).toBeVisible();
    await userEvent.click(screen.getAllByRole('button', { name: 'Close' }).at(-1)!);
    expect(screen.queryByRole('dialog')).toBeNull();
    act(() => useRecipeLibraryStore.getState().openUse({ ...target, revision: 9 }));
    await userEvent.click(screen.getByRole('button', { name: 'Use in project' }));
    expect(await screen.findByText(/revision r9/)).toBeVisible();
  });

  it('copies a starter with no revision into the project', async () => {
    await openMockLibrary('demo');
    useRecipeLibraryStore
      .getState()
      .openUse({ entry: 'starter:wave-hello', revision: null, name: 'Wave hello' });
    const used = vi.spyOn(commands, 'recipeLibraryUse');
    render(<UseRecipeSheet />);
    const sheet = screen.getByRole('dialog', { name: 'Use in project' });
    expect(sheet).toHaveTextContent('The starter into Frostveil');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Use in project' }));
    expect(used).toHaveBeenCalledWith({
      entry: 'starter:wave-hello',
      revision: null,
      project: 'frostveil',
      name: null,
    });
    expect(await within(sheet).findByText('Made Wave hello in Frostveil')).toBeVisible();
  });
});
