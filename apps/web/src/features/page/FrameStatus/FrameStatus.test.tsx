import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import type { McpStatus, ServerStatus } from '@/ipc/client';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import { FrameStatus } from '.';

const INITIAL = usePageStore.getState();

const STATUS: ServerStatus = {
  state: 'starting',
  project: 'frostveil',
  site: '/sites/frostveil-site',
  url: null,
  frameUrl: null,
  command: 'pnpm run dev --port 4321',
  owner: 'page',
  external: false,
  startedAt: null,
  message: null,
  log: ['$ pnpm run dev --port 4321', "Error: Cannot find module 'astro'"],
};

afterEach(() => {
  usePageStore.setState(INITIAL, true);
});

describe('the frame status', () => {
  it('shows a starting server with its command and log', () => {
    render(
      <FrameStatus
        state={{ kind: 'starting', status: STATUS }}
        site="frostveil-site"
        command="pnpm run dev"
      />
    );
    expect(screen.getByRole('heading', { name: "Starting the site's dev server" })).toBeVisible();
    expect(screen.getAllByText('pnpm run dev --port 4321')[0]).toBeVisible();
  });

  it('waits on the card instead of offering a start while a person is asked', () => {
    usePageStore.setState({ project: 'frostveil' });
    useAgentsStore.setState({
      status: {
        approvals: [{ kind: 'page-preview', project: 'frostveil' }],
      } as unknown as McpStatus,
    });
    render(<FrameStatus state={{ kind: 'idle' }} site="frostveil-site" command="pnpm run dev" />);
    expect(
      screen.getByRole('heading', { name: 'The dev server waits for your approval' })
    ).toBeVisible();
    expect(screen.queryByRole('button', { name: 'Start the dev server' })).toBeNull();
    useAgentsStore.setState({ status: null });
  });

  it('says why a server failed and offers to try again', async () => {
    const start = vi.fn(async () => undefined);
    usePageStore.setState({ start });
    render(
      <FrameStatus
        state={{
          kind: 'failed',
          status: { ...STATUS, state: 'failed', message: 'It stopped (exit code 1).' },
        }}
        site="frostveil-site"
        command={null}
      />
    );
    expect(screen.getByText('It stopped (exit code 1).')).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(start).toHaveBeenCalled();
  });

  it('points a project without a site repo at Settings or a page concept', async () => {
    render(<FrameStatus state={{ kind: 'no-site' }} site="No site" command={null} />);
    expect(screen.getByRole('heading', { name: 'No site repo yet' })).toBeVisible();
    await userEvent.click(screen.getByRole('button', { name: 'Open Settings' }));
    expect(useAppStore.getState().view).toBe('settings');
  });

  it('offers to take the server over from another project', () => {
    render(
      <FrameStatus state={{ kind: 'busy', project: 'elsheria-showcase' }} site="s" command={null} />
    );
    expect(
      screen.getByRole('heading', { name: 'The dev server serves elsheria-showcase' })
    ).toBeVisible();
    expect(screen.getByRole('button', { name: "Run this project's site" })).toBeVisible();
  });
});
