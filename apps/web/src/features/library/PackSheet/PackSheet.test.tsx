import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { NO_FILTERS } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibrarySelectionStore } from '@/state/use-library-selection-store';
import { useLibraryStore } from '@/state/use-library-store';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { follow, resetPacks } from '@/test/open-pack';
import LibraryView from '../LibraryView/LibraryView';
import PackSheet from './PackSheet';

vi.mock('@/features/viewer/ModelPreview', () => ({ default: () => null }));

afterEach(() => {
  vi.restoreAllMocks();
  clearMocks();
  resetPacks();
  useLibraryStore.setState({ view: null, filters: NO_FILTERS, selected: null, layout: 'grid' });
  useLibrarySelectionStore.getState().clear();
});

describe('export pack from the library', () => {
  it('packs the picked models: settings, then the check', async () => {
    await openMockLibrary('scale');
    render(
      <>
        <LibraryView />
        <PackSheet />
      </>
    );
    const filters = await screen.findByRole('region', { name: 'Filters' });
    await userEvent.click(within(filters).getByRole('button', { name: /^Village props/ }));
    const models = screen.getByRole('region', { name: 'Models' });
    await userEvent.click(within(models).getByRole('checkbox', { name: 'Select Bookshelf' }));
    await userEvent.click(within(models).getByRole('checkbox', { name: 'Select Cast iron stove' }));
    const bar = screen.getByRole('toolbar', { name: 'Selected models' });
    await userEvent.click(within(bar).getByRole('button', { name: 'Export pack' }));
    const sheet = await screen.findByRole('dialog', { name: 'Export pack' });
    await waitFor(() =>
      expect(within(sheet).getByText(/^2 models from the library, made ready/)).toBeVisible()
    );
    expect(within(sheet).getByRole('textbox', { name: 'Pack name' })).toHaveValue('library-pack');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Export and check' }));
    const id = usePackStore.getState().record!.id;
    await follow(id, ['done']);
    expect(await screen.findByRole('dialog', { name: 'library-pack is ready' })).toBeVisible();
  });

  it('packs the collection on show from the top bar', async () => {
    await openMockLibrary('scale');
    render(
      <>
        <LibraryView />
        <PackSheet />
      </>
    );
    const filters = await screen.findByRole('region', { name: 'Filters' });
    await userEvent.click(within(filters).getByRole('button', { name: /^Market set/ }));
    await userEvent.click(screen.getByRole('button', { name: 'Export pack' }));
    const sheet = await screen.findByRole('dialog', { name: 'Export pack' });
    expect(within(sheet).getByRole('textbox', { name: 'Pack name' })).toHaveValue('market-set');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Close' }));
    expect(usePackStore.getState().open).toBe(false);
  });

  it('gives focus back to what opened it on close', async () => {
    await openMockLibrary('scale');
    const id = useAppStore.getState().workspace!.project!.id;
    render(
      <>
        <button type="button">More</button>
        <PackSheet />
      </>
    );
    const opener = screen.getByRole('button', { name: 'More' });
    opener.focus();
    await act(() => usePackStore.getState().show({ from: 'project', project: id }));
    await userEvent.keyboard('{Escape}');
    await waitFor(() => expect(usePackStore.getState().open).toBe(false));
    await waitFor(() => expect(opener).toHaveFocus());
  });
});
