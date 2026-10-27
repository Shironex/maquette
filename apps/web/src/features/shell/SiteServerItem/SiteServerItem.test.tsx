import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { ServerStatus } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { SiteServerItem } from '.';

const running: ServerStatus = {
  state: 'running',
  project: 'frostveil',
  site: '/site',
  url: 'http://localhost:3000/',
  frameUrl: null,
  command: 'pnpm run dev:landing',
  owner: 'agent',
  external: false,
  startedAt: null,
  message: null,
  log: [],
};

afterEach(() => {
  clearMocks();
  usePageStore.setState({ server: null });
});

describe("the status bar's site dev server", () => {
  it('opens a menu that says who started it and stops it', async () => {
    await openMockWorkspace();
    usePageStore.setState({ server: running });
    render(<SiteServerItem label="Site dev page on localhost:3000" dot="ok" />);
    await userEvent.click(screen.getByRole('button', { name: /Site dev page on localhost:3000/ }));
    const menu = screen.getByRole('menu', { name: 'Site dev server' });
    expect(menu).toHaveTextContent('Frostveil, started by an agent');
    expect(menu).toHaveTextContent('pnpm run dev:landing');
    await userEvent.click(screen.getByRole('menuitem', { name: 'Stop the dev server' }));
    expect(screen.queryByRole('menu')).toBeNull();
    await vi.waitFor(() => expect(usePageStore.getState().server?.state).toBe('idle'));
  });

  it('goes to the Page view of the project it serves', async () => {
    await openMockWorkspace();
    usePageStore.setState({ server: running });
    render(<SiteServerItem label="Site dev page on localhost:3000" dot="ok" />);
    await userEvent.click(screen.getByRole('button', { name: /Site dev page/ }));
    await userEvent.click(screen.getByRole('menuitem', { name: 'Show in the Page view' }));
    expect(useAppStore.getState().view).toBe('page');
  });
});
