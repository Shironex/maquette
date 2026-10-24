import { describe, expect, it } from 'vitest';
import type { InboxItem, InboxSnapshot } from '@/ipc/client';
import { needsSnapshot, withThumbnail } from './thumbs';

const item = (patch: Partial<InboxItem>): InboxItem =>
  ({
    id: 'a.glb',
    file: '/w/inbox/a.glb',
    state: 'ready',
    thumbnail: null,
    thumbnailState: 'fallback',
    ...patch,
  }) as InboxItem;

describe('thumbnails', () => {
  it('snapshots only ready models that Blender will not render', () => {
    expect(needsSnapshot(item({}))).toBe(true);
    expect(needsSnapshot(item({ thumbnailState: 'rendering' }))).toBe(false);
    expect(needsSnapshot(item({ thumbnail: '/t.png', thumbnailState: 'ready' }))).toBe(false);
    expect(needsSnapshot(item({ state: 'failed' }))).toBe(false);
  });

  it('folds a finished render into the matching item only', () => {
    const snapshot = { items: [item({}), item({ id: 'b.glb', file: '/w/inbox/b.glb' })] };
    const next = withThumbnail(
      snapshot as unknown as InboxSnapshot,
      '/w/inbox/b.glb',
      '/t/b.png',
      'ready'
    );
    expect(next.items[0]).toBe(snapshot.items[0]);
    expect(next.items[1]).toMatchObject({ thumbnail: '/t/b.png', thumbnailState: 'ready' });
    expect(withThumbnail(snapshot as unknown as InboxSnapshot, '/nope', null, 'ready')).toBe(
      snapshot
    );
  });

  it('keeps the old thumbnail when a render fails', () => {
    const snapshot = { items: [item({ thumbnail: '/old.png', thumbnailState: 'ready' })] };
    const next = withThumbnail(
      snapshot as unknown as InboxSnapshot,
      '/w/inbox/a.glb',
      null,
      'fallback'
    );
    expect(next.items[0]).toMatchObject({ thumbnail: '/old.png', thumbnailState: 'fallback' });
  });
});
