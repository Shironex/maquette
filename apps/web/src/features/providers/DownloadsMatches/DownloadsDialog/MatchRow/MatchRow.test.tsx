import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { DownloadMatch } from '@/ipc/client';
import { useProvidersStore } from '@/state/use-providers-store';
import { clearMocks } from '@/test/mock-ipc';
import { openProviders } from '../../../openProviders';
import { MatchRow } from '.';

afterEach(() => {
  clearMocks();
  useProvidersStore.setState({ project: null, jobs: [], downloads: null });
});

const file: DownloadMatch = {
  path: '/Users/me/Downloads/lantern.zip',
  name: 'lantern.zip',
  inner: 'lantern.glb',
  bytes: 2_000_000,
  modifiedMs: 0,
  job: 'lanterns-1',
  assetName: 'Stone lanterns',
  reason: 'Named after the asset',
};

describe('a Downloads match row', () => {
  it('shows what the file holds and why it matched', async () => {
    const { job } = await openProviders();
    render(
      <ul>
        <MatchRow
          file={file}
          waiting={[job('lanterns-1')]}
          choice="lanterns-1"
          used={false}
          taken={[]}
          now={0}
        />
      </ul>
    );
    expect(screen.getByText(/^holds lantern\.glb · /)).toBeInTheDocument();
    expect(screen.getByText('Named after the asset')).toBeInTheDocument();
    expect(screen.getByRole('button', { name: 'Use' })).toBeEnabled();
  });

  it('cannot be used until a send is picked, and leaves out sends other rows took', async () => {
    const { job } = await openProviders();
    render(
      <ul>
        <MatchRow
          file={file}
          waiting={[job('lanterns-1')]}
          choice={null}
          used={false}
          taken={['lanterns-1']}
          now={0}
        />
      </ul>
    );
    expect(screen.getByRole('button', { name: 'Use' })).toHaveAttribute('aria-disabled', 'true');
    expect(screen.getAllByRole('option').map(o => o.textContent)).toEqual(['Not one of these']);
  });

  it('says which asset a used file was copied for', async () => {
    const { job } = await openProviders();
    render(
      <ul>
        <MatchRow
          file={file}
          waiting={[job('lanterns-1')]}
          choice="lanterns-1"
          used
          taken={[]}
          now={0}
        />
      </ul>
    );
    expect(screen.getByText(/Copied for Stone lanterns/)).toBeInTheDocument();
    expect(screen.queryByRole('button', { name: 'Use' })).toBeNull();
  });
});
