import { act, fireEvent, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/app-store';
import { useProjectsStore } from '@/state/projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { SettingsView } from './SettingsView';

afterEach(() => clearMocks());

describe('the project settings', () => {
  it('keep a switch that lands while the new name saves', async () => {
    const handle = await openMockWorkspace();
    let release = () => {};
    mockIPC(async (cmd, args) => {
      if (cmd === 'project_update') await new Promise<void>(r => (release = r));
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    render(<SettingsView />);
    const section = screen.getByRole('region', { name: /Project/ });
    const field = within(section).getByRole('textbox', { name: 'Name' });
    await userEvent.clear(field);
    await userEvent.type(field, 'Moon Island night');
    fireEvent.blur(field);

    await act(() => useProjectsStore.getState().switchTo('rooftop-cinema'));
    await act(async () => release());
    await waitFor(() =>
      expect(
        useAppStore.getState().workspace?.projects.find(p => p.id === 'moon-island')?.name
      ).toBe('Moon Island night')
    );
    expect(useAppStore.getState().workspace?.project?.id).toBe('rooftop-cinema');
  });
});

describe('leaving Settings with an invalid field', () => {
  async function openWithBadPreset() {
    await openMockWorkspace();
    useAppStore.setState({ view: 'settings' });
    render(<SettingsView />);
    const saved = useAppStore.getState().workspace!.config;
    const faces = screen.getAllByRole('textbox', { name: 'Face limit' })[0]!;
    await userEvent.clear(faces);
    await userEvent.type(faces, '30000{Enter}');
    // Too many faces for a P1 model.
    await userEvent.selectOptions(
      screen.getAllByRole('combobox', { name: 'Model' })[0]!,
      'P1-20260311'
    );
    return { saved };
  }

  it('asks, lists what is wrong, and Fix it stays', async () => {
    await openWithBadPreset();
    act(() => useAppStore.getState().setView('board'));
    const ask = await screen.findByRole('dialog', { name: 'Some settings are not valid' });
    expect(within(ask).getByText(/Tripo prop preset: .*takes a face limit/)).toBeVisible();
    await userEvent.click(within(ask).getByRole('button', { name: 'Fix it' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().view).toBe('settings');
  });

  it('Discard leaves, keeps the saved preset and still saves the valid edits', async () => {
    const { saved } = await openWithBadPreset();
    const sceneMb = (saved.budgets.sceneMb ?? 3) + 1;
    const field = screen.getByRole('textbox', { name: 'Scene target' });
    await userEvent.clear(field);
    await userEvent.type(field, `${sceneMb}{Enter}`);
    act(() => useAppStore.getState().setView('board'));
    await userEvent.click(
      within(await screen.findByRole('dialog')).getByRole('button', { name: 'Discard' })
    );
    expect(useAppStore.getState().view).toBe('board');
    await waitFor(() =>
      expect(useAppStore.getState().workspace?.config.budgets.sceneMb).toBe(sceneMb)
    );
    expect(useAppStore.getState().workspace?.config.providers.tripo.prop.faceLimit).toBe(
      saved.providers.tripo.prop.faceLimit
    );
  });

  it('does not ask when every field is valid', async () => {
    await openMockWorkspace();
    useAppStore.setState({ view: 'settings' });
    render(<SettingsView />);
    act(() => useAppStore.getState().setView('board'));
    expect(screen.queryByRole('dialog')).toBeNull();
    expect(useAppStore.getState().view).toBe('board');
  });
});
