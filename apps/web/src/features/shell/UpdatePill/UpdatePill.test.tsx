import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useAppStore } from '@/state/use-app-store';
import { useUpdaterStore } from '@/state/use-updater-store';
import { clearMocks, openMockUpdater } from '@/test/mock-ipc';
import { UpdatePill } from '.';

afterEach(() => clearMocks());

describe('the update pill', () => {
  it('stays hidden while the updater is inert', async () => {
    await openMockUpdater();
    render(<UpdatePill />);
    await waitFor(() => expect(useUpdaterStore.getState().mode).toBe('unreleased'));
    expect(screen.queryByRole('button')).toBeNull();
  });

  it('names the new version and opens Settings > About', async () => {
    const user = userEvent.setup();
    await openMockUpdater('available');
    useAppStore.setState({ view: 'board' });
    render(<UpdatePill />);
    await user.click(await screen.findByRole('button', { name: 'Update 0.2.0' }));
    expect(useAppStore.getState().view).toBe('settings');
    expect(useUpdaterStore.getState().aboutAsked).toBe(true);
  });

  it('keeps a name when the rail is collapsed', async () => {
    await openMockUpdater('ready');
    render(<UpdatePill collapsed />);
    expect(
      await screen.findByRole('button', { name: 'Maquette 0.2.0 is ready, restart to update' })
    ).toBeVisible();
  });
});
