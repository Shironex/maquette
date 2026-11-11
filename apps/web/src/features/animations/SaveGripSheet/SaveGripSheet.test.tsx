import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useGripLibraryStore } from '@/state/use-grip-library-store';
import { useGripSaveStore } from '@/state/use-grip-save-store';
import { clearMocks, openRecipe } from '@/test/open-recipe';
import SaveGripSheet from './SaveGripSheet';

afterEach(() => {
  clearMocks();
  useGripSaveStore.setState(useGripSaveStore.getInitialState(), true);
  useGripLibraryStore.setState(useGripLibraryStore.getInitialState(), true);
});

async function opened() {
  await openRecipe('scythe-charge-held');
  render(<SaveGripSheet />);
  await act(() => useGripSaveStore.getState().open('frostveil', 'scythe-charge-held', 'l-1', 30));
  return screen.getByRole('dialog', { name: 'Save a grip' });
}

describe('the save a grip sheet', () => {
  it('shows the pose, the bones by group and where the file goes', async () => {
    const sheet = await opened();
    expect(sheet).toHaveTextContent('From the pose keys on the hands at frame 30');
    expect(within(sheet).getByRole('img', { name: /The grip to save/ })).toBeVisible();
    expect(within(sheet).getByLabelText('Name')).toHaveValue('Scythe charge, held');
    const bones = within(sheet).getByRole('region', { name: 'Bones' });
    expect(bones).toHaveTextContent('Right fingers15');
    expect(bones).toHaveTextContent('Left fingers15');
    expect(sheet).toHaveTextContent(/library\/grips\/scythe-charge-held\/r1\.grip\.json · /);
    expect(within(sheet).getByRole('button', { name: 'Save grip' })).toBeEnabled();
  });

  it('says when a hand asked for is not keyed, and when the name is empty', async () => {
    const sheet = await opened();
    await userEvent.click(within(sheet).getByRole('checkbox', { name: 'Left hand' }));
    await userEvent.click(within(sheet).getByRole('checkbox', { name: 'Left fingers' }));
    expect(sheet).toHaveTextContent('The pose keys turn no bone on the left hand at frame 30');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Right holds' }));
    expect(within(sheet).getByRole('button', { name: 'Save grip' })).toBeEnabled();
    await userEvent.clear(within(sheet).getByLabelText('Name'));
    expect(sheet).toHaveTextContent('Give the grip a name.');
    expect(within(sheet).getByRole('button', { name: 'Save grip' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });

  it('saves a new grip and closes', async () => {
    const sheet = await opened();
    await userEvent.selectOptions(within(sheet).getByLabelText('Weapon'), 'scythe');
    await userEvent.click(within(sheet).getByRole('button', { name: 'Save grip' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    expect(
      useGripLibraryStore.getState().list?.find(g => g.id === 'scythe-charge-held')
    ).toMatchObject({ weapon: 'scythe', latest: 1 });
  });

  it('says what changes against the newest revision of a library grip', async () => {
    const sheet = await opened();
    await userEvent.selectOptions(within(sheet).getByLabelText('Save as'), 'scythe-staff-grip');
    expect(await within(sheet).findByText('What changes against r2')).toBeVisible();
    expect(within(sheet).getByLabelText('Name')).toHaveValue('Scythe, staff grip');
    expect(within(sheet).getByRole('button', { name: 'Save as r3' })).toBeEnabled();
  });
});
