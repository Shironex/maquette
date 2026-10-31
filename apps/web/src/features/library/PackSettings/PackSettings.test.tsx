import { render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { usePackStore } from '@/state/use-pack-store';
import { clearMocks, openMockLibrary } from '@/test/mock-ipc';
import { resetPacks } from '@/test/open-pack';
import PackSettings from './PackSettings';

afterEach(() => {
  clearMocks();
  resetPacks();
});

async function openSettings() {
  await openMockLibrary('scale');
  await usePackStore
    .getState()
    .show({ from: 'collection', collection: 'village-props' }, { collection: 'Village props' });
  render(<PackSettings />);
  await waitFor(() => expect(usePackStore.getState().plan).not.toBeNull());
}

describe('the settings of a pack', () => {
  it('starts from the built in preset, named after the collection', async () => {
    await openSettings();
    expect(screen.getByRole('textbox', { name: 'Pack name' })).toHaveValue('village-props');
    expect(screen.getByRole('combobox', { name: 'Preset' })).toHaveValue('plain-obj');
    const budgets = screen.getByRole('region', { name: 'Triangle budget per kind' });
    expect(within(budgets).getByRole('textbox', { name: 'Budget for props' })).toHaveValue('3,000');
    expect(screen.getByText(/^About .* · Blender runs each model/)).toBeVisible();
    expect(screen.queryByRole('combobox', { name: 'Compression' })).toBeNull();
    expect(screen.getByRole('button', { name: 'Export and check' })).toBeEnabled();
  });

  it('offers compression with GLB and drops a budget with its switch', async () => {
    await openSettings();
    await userEvent.click(screen.getByRole('button', { name: 'GLB' }));
    expect(screen.getByRole('combobox', { name: 'Compression' })).toHaveValue('none');
    await userEvent.click(screen.getByRole('switch', { name: 'A budget for props' }));
    expect(usePackStore.getState().settings?.budgets.find(b => b.kind === 'prop')?.triangles).toBe(
      null
    );
    expect(screen.getByText('As they are')).toBeVisible();
  });

  it('saves the settings as a preset', async () => {
    await openSettings();
    await userEvent.click(screen.getByRole('button', { name: 'Save as preset' }));
    await userEvent.type(screen.getByRole('textbox', { name: 'Preset name' }), 'For Godot');
    await userEvent.click(screen.getByRole('button', { name: 'Save' }));
    await waitFor(() =>
      expect(screen.getByRole('combobox', { name: 'Preset' })).toHaveValue('for-godot')
    );
    expect(screen.getByText('Your own, saved just now')).toBeVisible();
  });
});
