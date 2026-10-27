import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { PageView, ServerStatus } from '@/ipc/client';
import { usePageStore } from '@/state/use-page-store';
import { ServerControls } from '.';

const server: ServerStatus = {
  state: 'running',
  project: 'frostveil',
  site: '/site',
  url: 'http://localhost:3000/',
  frameUrl: null,
  command: 'pnpm run dev:landing',
  owner: 'pipeline',
  external: false,
  startedAt: null,
  message: null,
  log: [],
};

const view = (status: ServerStatus) =>
  ({ project: 'frostveil', server: status, settings: { source: 'site' } }) as PageView;

afterEach(() => usePageStore.setState({ server: null, view: null }));

describe("the Page view's server controls", () => {
  it('says who started the server when it was not the Page view, beside Stop', () => {
    usePageStore.setState({ server, view: view(server) });
    const { rerender } = render(<ServerControls />);
    expect(screen.getByText('started by Preview on site')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Stop' })).toBeInTheDocument();
    const own = { ...server, owner: 'page' as const };
    usePageStore.setState({ server: own, view: view(own) });
    rerender(<ServerControls />);
    expect(screen.queryByText(/^started by/)).toBeNull();
    expect(screen.getByRole('button', { name: 'Stop' })).toBeInTheDocument();
  });

  it('has nothing to stop while the server is stopped', () => {
    const idle = { ...server, state: 'idle' as const, owner: null };
    usePageStore.setState({ server: idle, view: view(idle) });
    render(<ServerControls />);
    expect(screen.queryByRole('button', { name: 'Stop' })).toBeNull();
  });
});
