import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NO_FILTERS } from '@/state/library';
import { useLibrarySelectionStore } from '@/state/use-library-selection-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryView from '../LibraryView/LibraryView';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ view: null, filters: NO_FILTERS, selected: null, layout: 'grid' });
  useLibrarySelectionStore.getState().clear();
});

async function openScale() {
  await openMockLibrary('scale');
  render(<LibraryView />);
  const filters = await screen.findByRole('region', { name: 'Filters' });
  await userEvent.click(within(filters).getByRole('button', { name: /^Village props/ }));
  return screen.getByRole('region', { name: 'Models' });
}

describe('bulk actions in the library', () => {
  it('picks models with their boxes and Shift, and hides the side panel meanwhile', async () => {
    const models = await openScale();
    expect(within(models).getByRole('heading', { name: 'Village props' })).toBeVisible();
    await userEvent.click(within(models).getByRole('checkbox', { name: 'Select Bookshelf' }));
    const user = userEvent.setup();
    await user.keyboard('{Shift>}');
    await user.click(within(models).getByRole('checkbox', { name: 'Select Cast iron stove' }));
    await user.keyboard('{/Shift}');
    const bar = screen.getByRole('toolbar', { name: 'Selected models' });
    expect(within(bar).getByText(/selected$/)).toBeVisible();
    expect(useLibrarySelectionStore.getState().selected.length).toBeGreaterThan(1);
    expect(screen.queryByRole('complementary', { name: 'Selected model' })).toBeNull();
    await userEvent.click(within(bar).getByRole('button', { name: 'Clear selection' }));
    expect(screen.queryByRole('toolbar', { name: 'Selected models' })).toBeNull();
  });

  it('tags every picked model from the Tag popover', async () => {
    const models = await openScale();
    await userEvent.click(within(models).getByRole('checkbox', { name: 'Select Bookshelf' }));
    await userEvent.click(within(models).getByRole('checkbox', { name: 'Select Farm table' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected models' });
    expect(within(bar).getByText('2 selected')).toBeVisible();
    await userEvent.click(within(bar).getByRole('button', { name: 'Tag' }));
    const pop = screen.getByRole('dialog', { name: 'Tag 2 models' });
    expect(
      within(pop).getByRole('button', { name: 'Put furniture on all 2, it is on 1' })
    ).toBeVisible();
    await userEvent.type(within(pop).getByRole('textbox', { name: 'Tag' }), 'Rustic{Enter}');
    await waitFor(() =>
      expect(within(pop).getByRole('button', { name: 'Take rustic off all 2' })).toBeVisible()
    );
    const view = useLibraryStore.getState().view!;
    const tags = (id: string) => view.items.find(i => i.entry.id === id)!.entry.tags;
    expect(tags('bookshelf')).toContain('rustic');
    expect(tags('farm-table')).toContain('rustic');
  });

  it('removes the picked models with an Undo that brings them back', async () => {
    const models = await openScale();
    await userEvent.click(within(models).getByRole('checkbox', { name: 'Select Bookshelf' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected models' });
    await userEvent.click(within(bar).getByRole('button', { name: 'Remove' }));
    await waitFor(() =>
      expect(useLibraryStore.getState().view!.items.some(i => i.entry.id === 'bookshelf')).toBe(
        false
      )
    );
    expect(useLibrarySelectionStore.getState().selected).toEqual([]);
  });
});
