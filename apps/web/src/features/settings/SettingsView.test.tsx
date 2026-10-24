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
