import { render, screen, waitFor } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import { DownloadsWatchFields } from '.';

afterEach(() => {
  clearMocks();
  useDownloadsStore.setState({ status: null, saving: false });
});

describe('Settings Downloads watch', () => {
  it('shows the watched folder and turns the watch off and on', async () => {
    await openMockWorkspace('configured', undefined, 'batch');
    render(<DownloadsWatchFields />);
    const toggle = await screen.findByRole('switch', { name: /Watch for By hand exports/ });
    await waitFor(() => expect(toggle).toBeChecked());
    expect(screen.getByLabelText('Folder')).toHaveValue('~/Downloads');
    expect(screen.getByText(/^Watching for \d+ By hand sends?\.$/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Use Downloads' })).toBeNull();

    await userEvent.click(toggle);
    await waitFor(() => expect(toggle).not.toBeChecked());
    expect(screen.getByText(/^Off\./)).toBeInTheDocument();
    expect(useDownloadsStore.getState().status?.custom).toBe(false);

    await userEvent.click(toggle);
    await waitFor(() => expect(toggle).toBeChecked());
  });

  it('goes back to Downloads after another folder was picked', async () => {
    await openMockWorkspace('configured');
    await useDownloadsStore.getState().setWatch(true, '/Users/maker/Exports');
    render(<DownloadsWatchFields />);
    expect(await screen.findByLabelText('Folder')).toHaveValue('~/Exports');
    await userEvent.click(screen.getByRole('button', { name: 'Use Downloads' }));
    await waitFor(() => expect(screen.getByLabelText('Folder')).toHaveValue('~/Downloads'));
  });
});
