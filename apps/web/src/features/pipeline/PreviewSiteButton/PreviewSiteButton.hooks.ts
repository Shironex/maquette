import { useCallback, useState } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { useAppStore } from '@/state/use-app-store';

/**
 * Starts the open project's site with its own dev server and opens it in
 * the browser. The first start can take a while; the button stays busy
 * until the site answers or the start fails.
 */
export function usePreviewSite() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const notify = useAppStore(s => s.notify);
  const [starting, setStarting] = useState(false);
  const id = project?.id ?? null;

  const preview = useCallback(async () => {
    if (!id) return;
    setStarting(true);
    try {
      const url = await commands.sitePreview(id);
      notify({ status: 'done', title: 'Opened the site preview', detail: url });
    } catch (error) {
      notify({ status: 'failed', title: 'Could not start the site', detail: errorMessage(error) });
    } finally {
      setStarting(false);
    }
  }, [id, notify]);

  return { hasSite: Boolean(project?.siteRepo), starting, preview };
}
