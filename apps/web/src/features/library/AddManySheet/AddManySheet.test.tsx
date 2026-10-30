import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAddManyStore } from '@/state/use-add-many-store';
import { useLibraryStore } from '@/state/use-library-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import AddManySheet from './AddManySheet';

afterEach(() => {
  clearMocks();
  useAddManyStore.setState({
    open: false,
    project: null,
    plan: null,
    result: null,
    progress: null,
  });
  useLibraryStore.setState({ view: null });
});

async function openSheet() {
  await openMockLibrary('scale');
  await useLibraryStore.getState().load();
  useAddManyStore.getState().show('hollow-vale');
  render(<AddManySheet />);
  return screen.findByRole('dialog', { name: 'Add 121 models from Hollow Vale' });
}

describe('adding a whole project', () => {
  it('shows the tags, the clashes with a choice each and the files that could not be read', async () => {
    const sheet = await openSheet();
    expect(within(sheet).getByRole('textbox', { name: 'Project tag' })).toHaveValue('hollow-vale');
    expect(within(sheet).getByText('4 to decide, 6 skipped')).toBeVisible();
    expect(
      within(sheet).getByRole('combobox', { name: 'What to do with Street lamp' })
    ).toHaveValue('version');
    expect(within(sheet).getByRole('combobox', { name: 'What to do with Stone well' })).toHaveValue(
      'rename'
    );
    expect(within(sheet).getByText('2 could not be read')).toBeVisible();
    expect(within(sheet).getByRole('button', { name: 'Add 113 models' })).toBeEnabled();
  });

  it('copies the models, tags them and says what it did', async () => {
    const sheet = await openSheet();
    await userEvent.selectOptions(
      within(sheet).getByRole('combobox', { name: 'What to do with Paper lanterns' }),
      'skip'
    );
    await userEvent.click(within(sheet).getByRole('button', { name: 'Add 112 models' }));
    await waitFor(() => expect(within(sheet).getByText(/^Added 112 models/)).toBeVisible(), {
      timeout: 5000,
    });
    const view = useLibraryStore.getState().view!;
    const lamp = view.items.find(i => i.entry.id === 'street-lamp')!;
    expect(lamp.entry.versions).toHaveLength(2);
    expect(view.items.find(i => i.entry.id === 'stone-well-3')?.entry.name).toBe('Stone well 3');
    expect(view.items.find(i => i.entry.id === 'mill-door')?.entry.tags).toEqual([
      'hollow-vale',
      'prop',
    ]);
  });
});
