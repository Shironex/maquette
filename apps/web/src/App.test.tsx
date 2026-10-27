import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { createBackend } from '@/ipc/mock/backend';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { App } from './App';

afterEach(() => {
  vi.useRealTimers();
  clearMocks();
  useAppStore.setState({ phase: 'booting', bootError: null, notice: null });
});

describe('the app', () => {
  it('opens onboarding when there is no workspace yet', async () => {
    await openMockWorkspace('first-run');
    useAppStore.setState({ phase: 'booting' });
    render(<App />);
    expect(await screen.findByRole('button', { name: 'Set up' })).toBeVisible();
  });

  it('asks before reopening a workspace another computer has open', async () => {
    const handle = createBackend({ scenario: 'configured', probeDelay: 0, pace: 0, sync: 'held' });
    mockIPC((cmd, args) => handle(cmd, (args ?? {}) as Record<string, unknown>), {
      shouldMockEvents: true,
    });
    useAppStore.setState({ phase: 'booting', workspace: null });
    const { unmount } = render(<App />);
    const dialog = await screen.findByRole('dialog', { name: 'Open this workspace here too?' });
    expect(dialog).toHaveTextContent('This workspace is open on Studio PC.');
    expect(useAppStore.getState().phase).toBe('onboarding');
    await userEvent.click(screen.getByRole('button', { name: 'Open anyway' }));
    await vi.waitFor(() => expect(useAppStore.getState().phase).toBe('app'));
    expect(useAppStore.getState().workspace?.project?.id).toBe('frostveil');
    // The shell stops listening before the IPC mock goes.
    unmount();
  });

  it('says when the backend does not answer and tries again', async () => {
    const handle = await openMockWorkspace('first-run');
    useAppStore.setState({ phase: 'booting' });
    let down = true;
    mockIPC((cmd, args) => {
      if (down && cmd === 'app_bootstrap') throw new Error('The worker is down.');
      return handle(cmd, (args ?? {}) as Record<string, unknown>);
    });
    render(<App />);
    expect(await screen.findByText('Maquette could not start')).toBeVisible();
    down = false;
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(await screen.findByRole('button', { name: 'Set up' })).toBeVisible();
  });

  it('dismisses a notice after five seconds', async () => {
    await openMockWorkspace('first-run');
    useAppStore.setState({ phase: 'booting' });
    render(<App />);
    await screen.findByRole('button', { name: 'Set up' });
    vi.useFakeTimers();
    act(() => useAppStore.getState().notify({ status: 'done', title: 'Cache cleared' }));
    expect(screen.getByText('Cache cleared')).toBeVisible();
    act(() => vi.advanceTimersByTime(4999));
    expect(screen.getByText('Cache cleared')).toBeVisible();
    act(() => vi.advanceTimersByTime(1));
    expect(screen.queryByText('Cache cleared')).toBeNull();
  });

  it('runs a notice button once and closes the notice', async () => {
    await openMockWorkspace('first-run');
    useAppStore.setState({ phase: 'booting' });
    render(<App />);
    await screen.findByRole('button', { name: 'Set up' });
    const run = vi.fn();
    act(() =>
      useAppStore.getState().notify({
        status: 'info',
        title: 'A new model in Downloads',
        action: { label: 'Choose', run },
      })
    );
    await userEvent.click(screen.getByRole('button', { name: 'Choose' }));
    expect(run).toHaveBeenCalledTimes(1);
    expect(screen.queryByText('A new model in Downloads')).toBeNull();
  });
});
