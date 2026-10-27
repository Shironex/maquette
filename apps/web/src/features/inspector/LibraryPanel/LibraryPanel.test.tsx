import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { InspectModel } from '@/ipc/client';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import LibraryPanel from './LibraryPanel';

const model = (asset: string) => ({ tweakAsset: asset }) as InspectModel;

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ view: null, adding: null, updating: null, plan: null });
});

describe('the Inspector library panel', () => {
  it('says where the model came from and offers the update waiting', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().load();
    render(<LibraryPanel project="frostveil" model={model('elsheria')} />);
    expect(screen.getByText(/From Elsheria v2 in the library. Update available/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Update to v3...' }));
    expect(useLibraryStore.getState().updating).toEqual({
      project: 'frostveil',
      asset: 'elsheria',
    });
  });

  it('offers Add to library for a model that is not in it', async () => {
    await openMockLibrary();
    await useLibraryStore.getState().load();
    render(<LibraryPanel project="frostveil" model={model('veil-platform')} />);
    expect(screen.getByText(/Not in the library/)).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Add to library...' }));
    expect(useLibraryStore.getState().adding).toEqual({
      project: 'frostveil',
      asset: 'veil-platform',
    });
  });
});
