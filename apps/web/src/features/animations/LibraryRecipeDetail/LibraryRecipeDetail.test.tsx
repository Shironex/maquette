import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useRecipeLibraryStore } from '@/state/use-recipe-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryRecipeDetail from './LibraryRecipeDetail';

afterEach(() => {
  clearMocks();
  useRecipeLibraryStore.setState(useRecipeLibraryStore.getInitialState(), true);
});

/** The demo library read and the panel on `id`; `refresh` shows it as the list now has it. */
async function openOn(id: string) {
  await openMockLibrary('demo');
  await useRecipeLibraryStore.getState().load();
  const panel = () => {
    const summary = useRecipeLibraryStore.getState().list?.find(e => e.id === id);
    return summary ? <LibraryRecipeDetail summary={summary} /> : <p>Gone</p>;
  };
  const view = render(panel());
  return { refresh: () => view.rerender(panel()) };
}

describe('a library recipe on show', () => {
  it('lists every revision newest first with where it came from', async () => {
    await openOn('reach-for-the-lantern');
    const revisions = within(screen.getByRole('region', { name: 'Revisions' })).getAllByRole(
      'button'
    );
    expect(revisions.map(r => r.textContent?.slice(0, 2))).toEqual(['r3', 'r2', 'r1']);
    expect(revisions[0]).toHaveTextContent('Saved from Frostveil / reach-for-the-lantern');
    expect(revisions[0]).toHaveAttribute('aria-pressed', 'true');
  });

  it("shows the picked revision's changes and blanks", async () => {
    await openOn('reach-for-the-lantern');
    const r3 = await screen.findByRole('region', { name: 'What r3 holds' });
    expect(await within(r3).findAllByRole('listitem')).not.toHaveLength(0);
    expect(within(r3).getByRole('list', { name: 'Changes' }).children).toHaveLength(4);
    await userEvent.click(screen.getByRole('button', { name: /^r1/ }));
    const r1 = await screen.findByRole('region', { name: 'What r1 holds' });
    expect(
      await within(r1)
        .findByRole('list', { name: 'Changes' })
        .then(l => l.children.length)
    ).toBe(3);
    expect(within(r1).getByRole('list', { name: 'Blanks' })).toHaveTextContent(
      'Prop · The prop the hand reaches'
    );
  });

  it('says where an imported revision came from and exports it', async () => {
    await openOn('wave-hello');
    expect(screen.getByRole('button', { name: /^r1/ })).toHaveTextContent(
      'Imported from wave-hello.recipe.json'
    );
    const exported = vi.spyOn(commands, 'recipeExport');
    await screen.findByRole('list', { name: 'Changes' });
    await userEvent.click(screen.getByRole('button', { name: 'Export' }));
    expect(exported).toHaveBeenCalledWith(
      { project: null, recipe: null, entry: 'wave-hello', revision: 1 },
      'Wave hello'
    );
    expect(useAppStore.getState().notice?.title).toBe('Exported wave-hello.recipe.json');
  });

  it('opens Use in project on the picked revision', async () => {
    await openOn('reach-for-the-lantern');
    await userEvent.click(screen.getByRole('button', { name: /^r2/ }));
    await screen.findByRole('region', { name: 'What r2 holds' });
    await screen.findByRole('list', { name: 'Changes' });
    await userEvent.click(screen.getByRole('button', { name: 'Use in project' }));
    expect(useRecipeLibraryStore.getState().using).toEqual({
      entry: 'reach-for-the-lantern',
      revision: 2,
      name: 'Reach for the lantern',
    });
  });

  it('removes a revision once confirmed, sending the version it read', async () => {
    const { refresh } = await openOn('reach-for-the-lantern');
    const version = useRecipeLibraryStore.getState().list![1]!.version;
    const removed = vi.spyOn(commands, 'recipeLibraryRemoveRevision');
    await userEvent.click(screen.getByRole('button', { name: 'Remove r3' }));
    const ask = screen.getByRole('dialog', { name: 'Remove r3?' });
    await userEvent.click(within(ask).getByRole('button', { name: 'Cancel' }));
    expect(removed).not.toHaveBeenCalled();
    await userEvent.click(screen.getByRole('button', { name: 'Remove r3' }));
    await userEvent.click(
      within(screen.getByRole('dialog', { name: 'Remove r3?' })).getByRole('button', {
        name: 'Remove r3',
      })
    );
    expect(removed).toHaveBeenCalledWith('reach-for-the-lantern', 3, version);
    await waitFor(() => expect(useRecipeLibraryStore.getState().list![1]!.latest).toBe(2));
    refresh();
    expect(await screen.findByRole('button', { name: 'Remove r2' })).toBeVisible();
    expect(screen.getByRole('button', { name: /^r2/ })).toHaveAttribute('aria-pressed', 'true');
  });

  it('says inline why the only revision stays, and removes the whole recipe', async () => {
    const { refresh } = await openOn('wave-hello');
    await userEvent.click(screen.getByRole('button', { name: 'Remove r1' }));
    await userEvent.click(
      within(screen.getByRole('dialog', { name: 'Remove r1?' })).getByRole('button', {
        name: 'Remove r1',
      })
    );
    expect(
      await screen.findByText(
        'r1 is the only revision of Wave hello. Remove the whole recipe instead.'
      )
    ).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Remove recipe' }));
    await userEvent.click(
      within(screen.getByRole('dialog', { name: 'Remove Wave hello?' })).getByRole('button', {
        name: 'Remove recipe',
      })
    );
    await waitFor(() => expect(useRecipeLibraryStore.getState().list).toHaveLength(2));
    refresh();
    expect(screen.getByText('Gone')).toBeInTheDocument();
  });
});
