import { act, render, screen, waitFor, within } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useUpdaterStore } from '@/state/use-updater-store';
import { clearMocks, openMockUpdater } from '@/test/mock-ipc';
import { UpdatesBlock } from '.';

let stop = () => {};
afterEach(() => {
  stop();
  clearMocks();
});

async function openWith(updater?: string) {
  await openMockUpdater(updater);
  stop = useUpdaterStore.getState().start();
  render(<UpdatesBlock version="0.1.0" />);
  await waitFor(() => expect(useUpdaterStore.getState().mode).not.toBeNull());
}

describe('the updates block', () => {
  it('shows the found update with its notes, downloads it, then waits for a restart', async () => {
    const user = userEvent.setup();
    await openWith('available');
    expect(screen.getByRole('status')).toHaveTextContent('Maquette 0.2.0 is out.');
    const notes = screen.getByRole('region', { name: 'Release notes for 0.2.0' });
    expect(within(notes).getByText(/Ship to site sends a finished scene/)).toBeVisible();
    expect(within(notes).getByText('2026-11-03')).toBeVisible();

    await user.click(screen.getByRole('button', { name: 'Download 0.2.0' }));
    expect(await screen.findByRole('progressbar')).toBeInTheDocument();
    await screen.findByRole('button', { name: 'Restart to update' }, { timeout: 4000 });
    expect(screen.getByRole('status')).toHaveTextContent('0.2.0 is ready. Restart to update.');
  });

  it('keeps the download and says why when a restart is refused', async () => {
    const user = userEvent.setup();
    await openWith('ready');
    await user.click(screen.getByRole('button', { name: 'Restart to update' }));
    expect(await screen.findByText(/pipeline run or a build is still going/)).toBeVisible();
    expect(useUpdaterStore.getState().phase.kind).toBe('ready');
  });

  it('checks now and says when this is the newest version', async () => {
    const user = userEvent.setup();
    await openWith('current');
    await user.click(screen.getByRole('button', { name: 'Check now' }));
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent('0.1.0 is the newest version.')
    );
  });

  it('treats a release that is not out yet as nothing to show', async () => {
    const user = userEvent.setup();
    await openWith('pending');
    await user.click(screen.getByRole('button', { name: 'Check now' }));
    await waitFor(() =>
      expect(screen.getByRole('status')).toHaveTextContent('No release is out for this computer')
    );
    expect(screen.queryByRole('button', { name: /Download/ })).toBeNull();
  });

  it('stays inert with no key: Check now is disabled and nothing is offered', async () => {
    await openWith();
    expect(screen.getByRole('status')).toHaveTextContent('first public release');
    expect(screen.getByRole('button', { name: 'Check now' })).toHaveAttribute(
      'aria-disabled',
      'true'
    );
    await act(() => useUpdaterStore.getState().check());
    expect(useUpdaterStore.getState().phase.kind).toBe('idle');
  });
});
