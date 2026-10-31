import { useState } from 'react';
import type { PackItem } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';

/** The picture to show for `item` now, and what to call when it will not load. */
export function usePackPicture(item: PackItem) {
  // A run writes its picture over the last one's, so its time keeps the URL new.
  const urls = [
    fileUrl(item.picture, item.tookMs ? String(item.tookMs) : undefined),
    fileUrl(item.model.thumbnail),
  ].filter((url): url is string => Boolean(url));
  const key = urls.join(' ');
  const [failed, setFailed] = useState({ key, count: 0 });
  return {
    url: urls[failed.key === key ? failed.count : 0],
    onError: () => setFailed(now => ({ key, count: now.key === key ? now.count + 1 : 1 })),
  };
}
