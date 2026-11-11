import { render, screen, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryGrips from './LibraryGrips';

afterEach(() => {
  clearMocks();
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

describe('the library grips', () => {
  it('says there are none yet and where one comes from', async () => {
    await openMockLibrary();
    render(<LibraryGrips />);
    expect(await screen.findByText('No grips yet')).toBeVisible();
    expect(screen.getByText(/Save a grip in the pose keys settings/)).toBeVisible();
  });

  it('lists the six demo grips, filters them and opens the picked one', async () => {
    await openMockLibrary('demo');
    render(<LibraryGrips />);
    const grid = await screen.findByRole('list', { name: 'Library grips' });
    expect(within(grid).getAllByRole('button')).toHaveLength(6);
    expect(screen.getByText('6 grips')).toBeVisible();
    expect(within(grid).getByRole('button', { name: 'Longsword, two hands' })).toHaveTextContent(
      'r3 · both hands · 33 bones'
    );
    expect(within(grid).getByRole('button', { name: 'Empty hand, relaxed' })).toHaveTextContent(
      'r4 · either hand · 16 bones'
    );
    await userEvent.click(screen.getByRole('button', { name: 'Shields' }));
    expect(within(grid).getAllByRole('button')).toHaveLength(1);
    await userEvent.click(within(grid).getByRole('button', { name: 'Shield, strap' }));
    expect(screen.getByRole('complementary', { name: 'Selected grip' })).toHaveTextContent(
      'Shield, strap'
    );
  });
});
