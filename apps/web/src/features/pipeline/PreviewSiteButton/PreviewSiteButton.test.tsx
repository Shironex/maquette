import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { MOCK_SITE_URL } from '@/ipc/mock/create-page-handlers';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import { useProjectsStore } from '@/state/use-projects-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { PreviewSiteButton } from '.';

afterEach(() => {
  clearMocks();
  usePageStore.setState({ server: null, view: null, project: null });
});

describe('Preview on site', () => {
  it('opens the site and says where', async () => {
    await openMockWorkspace();
    render(<PreviewSiteButton />);
    await userEvent.click(screen.getByRole('button', { name: 'Preview on site' }));
    await waitFor(() =>
      expect(useAppStore.getState().notice).toMatchObject({
        status: 'done',
        title: 'Opened the site preview',
        detail: MOCK_SITE_URL,
      })
    );
  });

  it('splits into Open preview and Stop while the server runs, and Stop ends it', async () => {
    await openMockWorkspace();
    render(<PreviewSiteButton />);
    await userEvent.click(screen.getByRole('button', { name: 'Preview on site' }));
    const open = await screen.findByRole('button', { name: 'Open preview' });
    expect(usePageStore.getState().server).toMatchObject({ state: 'running', owner: 'pipeline' });
    await userEvent.unhover(open);
    await userEvent.hover(open);
    expect((await screen.findByRole('tooltip')).textContent).toContain(
      'its dev server was started by Preview on site.'
    );

    await userEvent.click(screen.getByRole('button', { name: "Stop the site's dev server" }));
    expect(await screen.findByRole('button', { name: 'Preview on site' })).toBeTruthy();
    expect(screen.queryByRole('button', { name: 'Open preview' })).toBeNull();
    expect(usePageStore.getState().server?.state).toBe('idle');
  });

  it('shows Stop for a server another part of the app started', async () => {
    await openMockWorkspace();
    usePageStore.setState({
      server: {
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
      },
    });
    render(<PreviewSiteButton />);
    await userEvent.hover(screen.getByRole('button', { name: "Stop the site's dev server" }));
    expect(await screen.findByText('On localhost:3000, started by an agent.')).toBeInTheDocument();
  });

  it('waits for a site repo and says where to pick one', async () => {
    await openMockWorkspace();
    await useProjectsStore.getState().switchTo('frost-cathedral');
    render(<PreviewSiteButton />);
    const button = screen.getByRole('button', { name: 'Preview on site' });
    expect(button).toHaveAttribute('aria-disabled', 'true');
    await userEvent.hover(button);
    expect(await screen.findByText('Pick the site repo in Settings > Project first.')).toBeTruthy();
  });
});
