import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { RecipeStarter } from '@/ipc/client';
import { starterList } from '@/ipc/mock/recipe-starters';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import StarterDetail from './StarterDetail';

afterEach(() => {
  clearMocks();
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
});

const starter = (id: string): RecipeStarter => starterList([]).find(s => s.id === id)!;

describe('a starter on show', () => {
  it('says what it does, what it runs on, its changes and blanks', () => {
    render(<StarterDetail starter={starter('starter:wave-hello')} />);
    const panel = screen.getByRole('complementary', { name: 'Selected starter' });
    expect(panel).toHaveTextContent('Raises the right arm and waves the forearm');
    expect(panel).toHaveTextContent("Runs on the model's rest pose, so it needs no clip.");
    const changes = within(panel).getByRole('list', { name: 'Changes' });
    expect(changes).toHaveTextContent('Nudge · Right upper arm · 0% to 100%');
    expect(changes).toHaveTextContent('Wave · Right lower arm · 15% to 95%');
    expect(panel).toHaveTextContent('No blanks.');
  });

  it('says when no clip of its names is in the library and that its prop blank starts open', () => {
    render(<StarterDetail starter={starter('starter:reach-for-prop')} />);
    expect(screen.getByText(/Runs on a clip called Idle, Breathing Idle/)).toBeVisible();
    expect(screen.getByRole('list', { name: 'Blanks' })).toHaveTextContent(
      'Prop · The prop the hand reaches (open: pick a prop of the project)'
    );
  });

  it('opens Use in project on the starter, and saves an editable copy to the library', async () => {
    await openMockLibrary();
    render(<StarterDetail starter={starter('starter:lean-in')} />);
    await userEvent.click(screen.getByRole('button', { name: 'Use in project' }));
    expect(useRecipeLibraryStore.getState().using).toEqual({
      entry: 'starter:lean-in',
      revision: null,
      name: 'Lean in',
    });
    await userEvent.click(screen.getByRole('button', { name: 'Save to library' }));
    const added = useRecipeLibraryStore.getState().imported;
    expect(added).toMatchObject({ entry: 'lean-in', name: 'Lean in', starter: true });
    expect(useRecipeLibraryStore.getState().selected).toBe('lean-in');
    expect(useRecipeLibraryStore.getState().list?.map(e => e.id)).toEqual(['lean-in']);
  });
});
