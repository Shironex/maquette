import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryUpdateDialog from './LibraryUpdateDialog';

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ view: null, updating: null, plan: null, planError: null });
});

describe('Update from library', () => {
  it('shows what carries over and what no longer applies, then updates', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().openUpdate({ project: 'frostveil', asset: 'elsheria' });
    render(<LibraryUpdateDialog />);
    expect(screen.getByRole('dialog', { name: 'Update Elsheria to library v3' })).toBeVisible();
    expect(screen.getByText('Origin to bottom centre')).toBeVisible();
    expect(screen.getByText('Decimate to 60k')).toBeVisible();
    expect(screen.getByText(/already at or under 60k/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Update, no credits' }));
    expect(useLibraryStore.getState().updating).toBeNull();
  });

  it('closes without changing anything', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().openUpdate({ project: 'frostveil', asset: 'elsheria' });
    render(<LibraryUpdateDialog />);
    await userEvent.click(screen.getByRole('button', { name: 'Keep v2' }));
    expect(useLibraryStore.getState().updating).toBeNull();
  });
});
