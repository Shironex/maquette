import { render, screen } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import { afterEach, describe, expect, it } from 'vitest';
import type { Arrival } from '@/ipc/client';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { ArrivalRow } from '.';

afterEach(() => useDownloadsStore.setState({ choices: {}, busy: [] }));

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
  assetName: null,
  reason: '2 sends are waiting',
  inboxFile: null,
};

const options = [
  { value: 'lanterns-1', label: 'Lanterns' },
  { value: 'brush-cup-1', label: 'Brush cup' },
];

describe('a download asking for its asset', () => {
  it('shows what is inside and why, and adds it only once a send is picked', async () => {
    render(
      <ul>
        <ArrivalRow arrival={zip} options={options} now={60_000} />
      </ul>
    );
    expect(screen.getByText('tripo_model.zip')).toBeInTheDocument();
    expect(screen.getByText(/holds model\.glb/)).toBeInTheDocument();
    expect(screen.getByText('2 sends are waiting')).toBeInTheDocument();
    const add = screen.getByRole('button', { name: 'Add to inbox' });
    expect(add).toHaveAttribute('aria-disabled', 'true');
    await userEvent.selectOptions(
      screen.getByRole('combobox', { name: 'Asset for tripo_model.zip' }),
      'brush-cup-1'
    );
    expect(useDownloadsStore.getState().choices).toEqual({ a1: 'brush-cup-1' });
    expect(screen.getByRole('button', { name: 'Add to inbox' })).not.toHaveAttribute(
      'aria-disabled',
      'true'
    );
  });
});
