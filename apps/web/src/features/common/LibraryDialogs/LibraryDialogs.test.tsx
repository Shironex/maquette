import { act, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryDialogs from './LibraryDialogs';

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ adding: null, updating: null, plan: null });
});

describe('the library dialogs', () => {
  it('opens the one the store asks for', async () => {
    await openMockLibrary('demo');
    render(<LibraryDialogs />);
    expect(screen.queryByRole('dialog')).toBeNull();
    act(() => useLibraryStore.getState().openAdd('pick'));
    expect(screen.getByRole('dialog', { name: 'Add to library' })).toBeVisible();
    act(() => useLibraryStore.getState().closeAdd());
    await act(() =>
      useLibraryStore.getState().openUpdate({ project: 'moon-island', asset: 'character' })
    );
    expect(screen.getByRole('dialog', { name: 'Update Character to library v3' })).toBeVisible();
  });
});
