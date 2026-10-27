import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { createBackend } from '@/ipc/mock/backend';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { useSyncConflicts } from '@/state/use-sync-conflicts';
import { clearMocks } from '@/test/mock-ipc';
import { SyncConflictBanner } from '.';

afterEach(() => {
  clearMocks();
  useSyncConflicts.setState({ conflicts: [], dismissed: null });
});

async function open(sync: string | null) {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, sync });
  const revealed: string[] = [];
  mockIPC((cmd, args) => {
    const a = (args ?? {}) as Record<string, unknown>;
    if (cmd === 'workspace_reveal') revealed.push(String(a.path));
    return handle(cmd, a);
  });
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
  return revealed;
}

describe('the sync conflict warning', () => {
  it('stays away while there are no copies', async () => {
    await open(null);
    render(<SyncConflictBanner />);
    await useSyncConflicts.getState().refresh();
    expect(screen.queryByRole('status')).toBeNull();
  });

  it('names each copy, shows it in the file manager and can be dismissed', async () => {
    const revealed = await open('conflict');
    render(<SyncConflictBanner />);
    const banner = await screen.findByRole('status');
    expect(banner).toHaveTextContent('Syncthing kept 2 conflict copies of Maquette files');
    expect(banner).toHaveTextContent('projects/frostveil/project.json has a copy');
    expect(banner).toHaveTextContent('maquette.json has a copy');
    const shows = screen.getAllByRole('button', { name: 'Reveal copy in Finder' });
    await userEvent.click(shows[0]!);
    expect(revealed).toEqual([
      'projects/frostveil/project.sync-conflict-20261027-101500-PCSTUDIO.json',
    ]);
    await userEvent.click(screen.getByRole('button', { name: 'Dismiss' }));
    expect(screen.queryByRole('status')).toBeNull();
  });
});
