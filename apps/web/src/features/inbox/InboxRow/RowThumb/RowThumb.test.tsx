import { render } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import type { InboxItem } from '@/ipc/client';
import { registerMockFile } from '@/ipc/files';
import { useInboxStore } from '@/state/use-inbox-store';
import { clearMocks, openMockWorkspace } from '@/test/mock-ipc';
import RowThumb from './RowThumb';

beforeEach(async () => {
  await openMockWorkspace();
  await useInboxStore.getState().load();
});

afterEach(() => {
  clearMocks();
  useInboxStore.setState({ snapshot: null, selected: null });
});

function inboxItem(id: string): InboxItem {
  const item = useInboxStore.getState().snapshot?.items.find(i => i.id === id);
  if (!item) throw new Error(`No mock inbox item ${id}`);
  return item;
}

describe('RowThumb', () => {
  it('shows the studio render once there is one', () => {
    const item = inboxItem('sakura_large.glb');
    registerMockFile(item.thumbnail ?? '', '/mock/sakura.webp');
    const { container } = render(<RowThumb item={item} />);
    expect(container.querySelector('img')).toHaveAttribute('src', '/mock/sakura.webp');
  });

  it('pulses the kind icon while the render is on its way', () => {
    const item = { ...inboxItem('character.glb'), thumbnail: null, thumbnailState: 'rendering' };
    const { container } = render(<RowThumb item={item as InboxItem} />);
    expect(container.firstElementChild).toHaveAttribute('title', 'Rendering a thumbnail');
    expect(container.querySelector('svg')).toHaveClass('animate-pulse');
  });

  it('shows no render state for a file that cannot be read', () => {
    const { container } = render(<RowThumb item={inboxItem('far_spire.glb')} />);
    expect(container.querySelector('img')).toBeNull();
    expect(container.firstElementChild).not.toHaveAttribute('title');
  });
});
