/** Thumbnails in the UI: kind placeholders and folding in finished renders. */
import type { IconName } from '@maquette/ui';
import type { AssetKind, InboxItem, InboxSnapshot, ThumbnailReady } from '@/ipc/client';

/** What stands in for a model with no thumbnail yet. */
export const KIND_ICON: Record<AssetKind, IconName> = {
  prop: 'cube',
  character: 'person-simple',
  scene: 'mountains',
};

/** True when Blender will not render this item, so the preview must snapshot it. */
export function needsSnapshot(item: InboxItem): boolean {
  return item.state === 'ready' && !item.thumbnail && item.thumbnailState === 'fallback';
}

/** The inbox with a finished render (or a stored snapshot) folded in. */
export function withThumbnail(
  snapshot: InboxSnapshot,
  file: string,
  thumbnail: string | null,
  state: ThumbnailReady['state']
): InboxSnapshot {
  if (!snapshot.items.some(i => i.file === file)) return snapshot;
  return {
    ...snapshot,
    items: snapshot.items.map(i =>
      i.file === file ? { ...i, thumbnail: thumbnail ?? i.thumbnail, thumbnailState: state } : i
    ),
  };
}

/**
 * A fresh listing with the thumbnails already known for unchanged files
 * kept: a listing taken while a render was still running says "rendering"
 * or "fallback", and the `thumbnailReady` that followed must win.
 */
export function keepRenders(before: InboxSnapshot, listed: InboxSnapshot): InboxSnapshot {
  const known = new Map(
    before.items
      .filter(i => i.thumbnail && i.thumbnailState === 'ready')
      .map(i => [i.file, i] as const)
  );
  if (known.size === 0) return listed;
  return {
    ...listed,
    items: listed.items.map(item => {
      const old = known.get(item.file);
      const same = old && old.bytes === item.bytes && old.modifiedMs === item.modifiedMs;
      return same && !item.thumbnail
        ? { ...item, thumbnail: old.thumbnail, thumbnailState: 'ready' as const }
        : item;
    }),
  };
}
