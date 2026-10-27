import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../openProviders';
import { DownloadsDialog } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], downloads: null });
});

const fresh = { project: 'frostveil', choices: {}, used: [] };

describe('the Downloads dialog', () => {
  it('offers a retry when the folder could not be read', async () => {
    const { sent } = await openProviders();
    useProvidersStore.setState({ downloads: { ...fresh, scan: null, error: 'No such folder' } });
    render(<DownloadsDialog onClose={() => {}} />);
    expect(screen.getByText('Could not read the folder')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(sent('provider_check_downloads')).toHaveLength(1);
  });

  it('says when no send waits for a model, and closes on Escape', async () => {
    const onClose = vi.fn();
    useProvidersStore.setState({
      downloads: {
        ...fresh,
        error: null,
        scan: { folder: '/Users/me/Downloads', sinceMs: null, files: [], waiting: [] },
      },
    });
    render(<DownloadsDialog onClose={onClose} />);
    expect(screen.getByText('No By hand send is waiting for a model.')).toBeInTheDocument();
    expect(screen.getByText(/0 files, 0 matched$/)).toBeInTheDocument();
    await userEvent.keyboard('{Escape}');
    expect(onClose).toHaveBeenCalledOnce();
  });
});
