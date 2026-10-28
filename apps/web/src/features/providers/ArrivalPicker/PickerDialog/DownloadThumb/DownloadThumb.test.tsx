import { render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import type { Arrival } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { DownloadThumb } from '.';

afterEach(() => useDownloadsStore.setState({ previews: {} }));

const arrival: Arrival = {
  id: 'thumb-1',
  path: '/Users/maker/Downloads/lantern post 3d model.glb',
  name: 'lantern post 3d model.glb',
  inner: null,
  bytes: 1000,
  atMs: 0,
  state: 'review',
  job: 'lantern-post-1',
  project: 'frostveil',
  asset: 'lantern-post',
  assetName: 'Lantern post',
  reason: 'Name close to Lantern post',
  inboxFile: null,
};

describe('a download thumbnail', () => {
  it('shows the headless render once it is there', () => {
    registerMockFile('/cache/thumbs/lantern.png', 'blob:lantern');
    useDownloadsStore.setState({
      previews: {
        'thumb-1': {
          file: '/data/download-previews/thumb-1.glb',
          thumbnail: '/cache/thumbs/lantern.png',
          state: 'ready',
        },
      },
    });
    render(<DownloadThumb arrival={arrival} />);
    expect(
      screen.getByRole('img', { name: 'The model in lantern post 3d model.glb' })
    ).toHaveAttribute('src', 'blob:lantern');
  });

  it('keeps its file icon while the render is on its way', () => {
    useDownloadsStore.setState({
      previews: { 'thumb-1': { file: '/x.glb', thumbnail: null, state: 'rendering' } },
    });
    const { container } = render(<DownloadThumb arrival={arrival} />);
    expect(screen.queryByRole('img')).toBeNull();
    expect(container.firstElementChild).toHaveAttribute('title', 'Rendering a thumbnail');
  });
});
