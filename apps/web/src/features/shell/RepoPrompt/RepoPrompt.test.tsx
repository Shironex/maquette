import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it } from 'vitest';
import { createBackend } from '@/ipc/mock/backend';
import { commands } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { clearMocks } from '@/test/mock-ipc';
import { RepoPrompt } from '.';

afterEach(() => {
  clearMocks();
  window.localStorage.clear();
});

async function open(sync: string | null, picked: string | null = '/Users/me/moon-island-site') {
  const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, sync });
  mockIPC((cmd, args) => {
    if (cmd === 'dialog_pick_folder') return picked;
    return handle(cmd, (args ?? {}) as Record<string, unknown>);
  });
  const boot = await commands.appBootstrap();
  useAppStore.setState({ boot, workspace: boot.workspace, phase: 'app' });
}

const question = "Where is this project's repo on this computer?";

describe('the repo location prompt', () => {
  it('stays away when this computer knows the repo', async () => {
    await open(null);
    render(<RepoPrompt />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('asks for a repo picked on another computer and saves the answer', async () => {
    await open('repo');
    render(<RepoPrompt />);
    const dialog = screen.getByRole('dialog', { name: question });
    expect(dialog).toHaveTextContent('(moon-island-site) was picked on another computer');
    expect(screen.getByRole('button', { name: 'Choose folder' })).toHaveFocus();
    await userEvent.click(screen.getByRole('button', { name: 'Choose folder' }));
    await waitFor(() => expect(screen.queryByRole('dialog')).toBeNull());
    const project = useAppStore.getState().workspace?.project;
    expect(project?.siteRepo).toBe('/Users/me/moon-island-site');
    expect(project?.siteRepoState).toBe('here');
  });

  it('remembers Not now for this project', async () => {
    await open('repo');
    const first = render(<RepoPrompt />);
    await userEvent.click(screen.getByRole('button', { name: 'Not now' }));
    expect(screen.queryByRole('dialog')).toBeNull();
    first.unmount();
    render(<RepoPrompt />);
    expect(screen.queryByRole('dialog')).toBeNull();
  });

  it('stays open when the folder picker is cancelled', async () => {
    await open('repo', null);
    render(<RepoPrompt />);
    await userEvent.click(screen.getByRole('button', { name: 'Choose folder' }));
    expect(screen.getByRole('dialog', { name: question })).toBeInTheDocument();
  });
});
