import { act, cleanup, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { useRigStore } from '@/state/use-rig-store';
import type { RigRecord } from '@/ipc/client';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { AppShell } from '.';

afterEach(async () => {
  // Unmount first and let the listeners unregister through the mock.
  cleanup();
  await act(async () => undefined);
  clearMocks();
  window.localStorage.clear();
  usePipelineStore.setState({ view: null });
  useProjectsStore.setState({ list: [], menuOpen: false, dialog: null });
});

async function renderShell() {
  const handle = await openMockWorkspace();
  // The shell listens to the backend's events.
  mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
    shouldMockEvents: true,
  });
  useAppStore.setState({ view: 'settings' });
  render(<AppShell />);
  return screen.getByRole('navigation', { name: 'Main' });
}

describe('the app shell', () => {
  it('shows the Inbox list from the rail, even with a rig doctor case left open', async () => {
    const rail = await renderShell();
    const record = { id: 'case-1' } as unknown as RigRecord;
    useRigStore.setState({ record });
    await userEvent.click(within(rail).getByRole('button', { name: /Inbox/ }));
    expect(useAppStore.getState().view).toBe('inbox');
    expect(useRigStore.getState().record).toBeNull();
    // Another view leaves the case where it was.
    useRigStore.setState({ record });
    await userEvent.click(within(rail).getByRole('button', { name: /Pipeline/ }));
    expect(useRigStore.getState().record).toBe(record);
    useRigStore.setState({ record: null });
  });

  it('shows the selected view and switches views from the rail', async () => {
    const rail = await renderShell();
    expect(
      await screen.findByRole('heading', { level: 1, name: 'Settings' }, { timeout: 4000 })
    ).toBeVisible();
    await userEvent.click(within(rail).getByRole('button', { name: /Pipeline/ }));
    expect(useAppStore.getState().view).toBe('pipeline');
  });

  it('remembers a collapsed rail on this machine', async () => {
    const rail = await renderShell();
    await userEvent.click(within(rail).getByRole('button', { name: 'Collapse the sidebar' }));
    expect(within(rail).getByRole('button', { name: 'Expand the sidebar' })).toBeVisible();
    expect(window.localStorage.getItem('maquette.rail.collapsed')).toBe('1');
    expect(within(rail).queryByText('maquette')).toBeNull();
  });

  it('opens the project switcher from the keyboard', async () => {
    await renderShell();
    await userEvent.keyboard('{Control>}p{/Control}');
    expect(await screen.findByRole('menu', { name: 'Projects' })).toBeVisible();
    await waitFor(() => expect(useProjectsStore.getState().menuOpen).toBe(true));
  });

  it('starts the status bar with the local worker', async () => {
    await renderShell();
    expect(screen.getByText('Local worker connected')).toBeVisible();
  });
});
