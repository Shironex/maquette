import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { ServerStatus } from '@/ipc/client';
import { usePageStore } from '@/state/use-page-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { StopServerButton } from '.';

const running: ServerStatus = {
  state: 'running',
  project: 'frostveil',
  site: '/site',
  url: 'http://localhost:3000/',
  frameUrl: null,
  command: 'pnpm run dev:landing',
  owner: 'page',
  external: false,
  startedAt: null,
  message: null,
  log: [],
};

afterEach(() => {
  clearMocks();
  usePageStore.setState({ server: null });
});

describe('Stop for the site dev server', () => {
  it('shows only while the server runs for the project', () => {
    usePageStore.setState({ server: running });
    const { rerender } = render(<StopServerButton project="rooftop-cinema" />);
    expect(screen.queryByRole('button', { name: 'Stop' })).toBeNull();
    rerender(<StopServerButton project="frostveil" />);
    expect(screen.getByRole('button', { name: 'Stop' })).toBeInTheDocument();
    usePageStore.setState({ server: { ...running, state: 'idle' } });
    rerender(<StopServerButton project="frostveil" />);
    expect(screen.queryByRole('button', { name: 'Stop' })).toBeNull();
  });

  it('says where and who, and stops the server', async () => {
    await openMockWorkspace();
    await usePageStore.getState().open('frostveil');
    usePageStore.setState({ server: running });
    render(<StopServerButton project="frostveil" />);
    const stop = screen.getByRole('button', { name: 'Stop' });
    await userEvent.hover(stop);
    expect((await screen.findByRole('tooltip')).textContent).toContain(
      "Stops the site's dev server on localhost:3000, started by the Page view."
    );
    await userEvent.click(stop);
    expect(usePageStore.getState().server?.state).toBe('idle');
    expect(screen.queryByRole('button', { name: 'Stop' })).toBeNull();
  });

  it('only forgets a server Maquette did not start', async () => {
    usePageStore.setState({ server: { ...running, external: true } });
    render(<StopServerButton project="frostveil" compact />);
    await userEvent.hover(screen.getByRole('button', { name: "Stop the site's dev server" }));
    expect((await screen.findByRole('tooltip')).textContent).toContain(
      'Maquette did not start it, so Stop only forgets it.'
    );
  });
});
