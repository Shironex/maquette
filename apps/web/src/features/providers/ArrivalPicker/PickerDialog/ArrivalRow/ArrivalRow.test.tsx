import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { Arrival } from '@/ipc/client';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { ArrivalRow } from '.';

afterEach(() => useDownloadsStore.setState({ chooserFor: null, previews: {}, busy: [] }));

const zip: Arrival = {
  id: 'a1',
  path: '/Users/maker/Downloads/tripo_model.zip',
  name: 'tripo_model.zip',
  inner: 'model.glb',
  bytes: 2_500_000,
  atMs: 0,
  state: 'unmatched',
  job: null,
  project: null,
  asset: null,
  assetName: null,
  reason: '2 sends are waiting',
  inboxFile: null,
};

describe('a download asking for its asset', () => {
  it('shows what is inside and why, and opens the asset chooser for it', async () => {
    render(
      <ul>
        <ArrivalRow arrival={zip} now={60_000} />
      </ul>
    );
    expect(screen.getByText('tripo_model.zip')).toBeInTheDocument();
    expect(screen.getByText(/holds model\.glb/)).toBeInTheDocument();
    expect(screen.getByText('2 sends are waiting')).toBeInTheDocument();
    await userEvent.click(screen.getByRole('button', { name: 'Attach to an asset' }));
    expect(useDownloadsStore.getState().chooserFor).toBe('a1');
  });
});
