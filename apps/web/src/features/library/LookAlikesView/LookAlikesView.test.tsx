import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NO_FILTERS } from '@/state/library';
import { useLibraryStore } from '@/state/use-library-store';
import { useLookAlikesStore } from '@/state/use-look-alikes-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryView from '../LibraryView/LibraryView';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ view: null, filters: NO_FILTERS, selected: null, layout: 'grid' });
  useLookAlikesStore.setState({ report: null });
});

describe('look-alikes', () => {
  it('flags the pairs on the cards and opens the list from the summary', async () => {
    await openMockLibrary('scale');
    render(<LibraryView />);
    const models = await screen.findByRole('region', { name: 'Models' });
    await userEvent.click(
      await within(models).findByRole('button', { name: '3 look-alike groups' })
    );
    const list = screen.getByRole('region', { name: 'Look-alikes' });
    expect(
      within(list).getByRole('listitem', { name: 'Street lamp and Street lamp 2' })
    ).toBeVisible();
    expect(within(list).getByRole('listitem', { name: 'Bee hive and Hive box' })).toBeVisible();
    // The third waits folded until Review.
    await userEvent.click(within(list).getByRole('button', { name: 'Review' }));
    expect(within(list).getByRole('button', { name: 'Rename both' })).toBeVisible();
    const side = screen.getByRole('complementary', { name: 'How look-alikes are found' });
    expect(within(side).getByText('2 models have no picture')).toBeVisible();
  });

  it('merges a pair as versions, lists the merge and undoes it', async () => {
    await openMockLibrary('scale');
    useLibraryStore.setState({ layout: 'look-alikes' });
    render(<LibraryView />);
    const list = await screen.findByRole('region', { name: 'Look-alikes' });
    const pair = await within(list).findByRole('listitem', {
      name: 'Street lamp and Street lamp 2',
    });
    await userEvent.click(within(pair).getByRole('button', { name: 'Merge as versions' }));
    await waitFor(() =>
      expect(
        within(list).queryByRole('listitem', { name: 'Street lamp and Street lamp 2' })
      ).toBeNull()
    );
    const merged = within(list)
      .getByText(/Street lamp 2 became/)
      .closest('li')!;
    await userEvent.click(within(merged).getByRole('button', { name: 'Undo' }));
    await waitFor(() =>
      expect(
        within(list).getByRole('listitem', { name: 'Street lamp and Street lamp 2' })
      ).toBeVisible()
    );
  });

  it('keeps both of a pair and hides it', async () => {
    await openMockLibrary('scale');
    useLibraryStore.setState({ layout: 'look-alikes' });
    render(<LibraryView />);
    const list = await screen.findByRole('region', { name: 'Look-alikes' });
    const pair = await within(list).findByRole('listitem', { name: 'Bee hive and Hive box' });
    await userEvent.click(within(pair).getByRole('button', { name: 'Keep both' }));
    await waitFor(() =>
      expect(within(list).queryByRole('listitem', { name: 'Bee hive and Hive box' })).toBeNull()
    );
  });
});
