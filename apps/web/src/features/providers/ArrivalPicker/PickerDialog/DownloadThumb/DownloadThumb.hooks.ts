import { useEffect } from 'react';
import type { Arrival } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useDownloadsStore } from '@/state/use-downloads-store';

/** The download's thumbnail URL, asked for once it shows; null while there is none. */
export function useDownloadThumb(arrival: Arrival) {
  const preview = useDownloadsStore(s => s.previews[arrival.id]);
  const loadPreview = useDownloadsStore(s => s.loadPreview);
  useEffect(() => {
    void loadPreview(arrival.id);
  }, [arrival.id, loadPreview]);
  return {
    url: fileUrl(preview?.thumbnail),
    rendering: preview?.state === 'rendering',
  };
}
