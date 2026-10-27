import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import AddToLibraryDialog from './AddToLibraryDialog';
import { parseTags } from './AddToLibraryDialog.hooks';

afterEach(() => {
  clearMocks();
  useLibraryStore.setState({ view: null, adding: null, selected: null });
});

describe('Add to library', () => {
  it('adds a fixed asset with its name and tags', async () => {
    await openMockLibrary();
    useLibraryStore.setState({ adding: { project: 'frostveil', asset: 'veil-platform' } });
    render(<AddToLibraryDialog target={{ project: 'frostveil', asset: 'veil-platform' }} />);
    expect(screen.getByRole('dialog', { name: 'Add Veil platform to the library' })).toBeVisible();
    await userEvent.type(screen.getByLabelText('Tags'), 'scenes, Night');
    await userEvent.click(screen.getByRole('button', { name: 'Add to library' }));
    expect(useAppStore.getState().notice?.title).toBe('Added Veil platform v1 to the library');
    expect(useLibraryStore.getState().view?.items[0]?.entry.tags).toEqual(['scenes', 'night']);
  });

  it('picks a project and one of its assets with a model', async () => {
    await openMockLibrary();
    render(<AddToLibraryDialog target="pick" />);
    expect(screen.getByRole('dialog', { name: 'Add to library' })).toBeVisible();
    expect(screen.getByLabelText('Name in the library')).toHaveValue('Veil platform');
  });

  it('says a newer model of a library asset becomes a new version', async () => {
    await openMockLibrary('demo');
    await useLibraryStore.getState().load();
    render(<AddToLibraryDialog target={{ project: 'frostveil', asset: 'observatory-arch' }} />);
    expect(
      screen.getByText(/came from the library's Observatory arch. A newer model adds a version/)
    ).toBeVisible();
    expect(screen.getByRole('button', { name: 'Add as a new version' })).toBeVisible();
  });

  it('splits tags on commas', () => {
    expect(parseTags(' props, ,desk ')).toEqual(['props', 'desk']);
  });
});
