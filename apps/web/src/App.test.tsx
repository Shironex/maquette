import { act, render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { mockIPC } from '@tauri-apps/api/mocks';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
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
});
