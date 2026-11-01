import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { commands } from '@/ipc/client';
import { NO_FILTERS } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibrarySelectionStore } from '@/state/use-library-selection-store';
import { useLibraryStore } from '@/state/use-library-store';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { follow, resetPacks, runPack } from '@/test/open-pack';
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

  /** Another project's pack, waiting for an answer, with the sheet mounted. */
  async function othersPack() {
    const record = await runPack(3, 'others-pack');
    const other = { ...record, source: { from: 'project' as const, project: 'someone-else' } };
    vi.spyOn(commands, 'packGet').mockResolvedValue(other);
    render(<PackSheet />);
    return other;
  }

  it('stays open on the recent pack of another project', async () => {
    const pack = await othersPack();
    await act(() => usePackStore.getState().showRecord(pack.id));
    await waitFor(() => expect(usePackStore.getState().record?.id).toBe(pack.id));
    expect(usePackStore.getState().open).toBe(true);
  });

  it('stays open on resuming the pack of another project', async () => {
    const pack = await othersPack();
    vi.spyOn(commands, 'packResume').mockResolvedValue(pack);
    await act(() => usePackStore.getState().resume(pack.id));
    await waitFor(() => expect(usePackStore.getState().record?.id).toBe(pack.id));
    expect(usePackStore.getState().open).toBe(true);
  });

  it('closes when another project opens, and the pack can still be answered after', async () => {
    const pack = await runPack(3, 'left-behind');
    render(<PackSheet />);
    expect(usePackStore.getState().open).toBe(true);
    const workspace = useAppStore.getState().workspace!;
    act(() =>
      useAppStore.setState({
        workspace: { ...workspace, project: { ...workspace.project!, id: 'other' } },
      })
    );
    await waitFor(() => expect(usePackStore.getState().open).toBe(false));
    await act(() => usePackStore.getState().showRecord(pack.id));
    expect(usePackStore.getState()).toMatchObject({ open: true });
    expect(usePackStore.getState().record?.id).toBe(pack.id);
    const decide = vi.spyOn(commands, 'packDecide').mockResolvedValue(pack);
    await act(() => usePackStore.getState().decide({} as never));
    expect(decide).toHaveBeenCalledWith(pack.id, {});
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
