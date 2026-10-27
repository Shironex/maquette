import { useCallback, useEffect, useState } from 'react';
import { commands, errorMessage } from '@/ipc/client';
import { serverHost, serverLive, serverOwner } from '@/state/page';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';

/**
 * Starts the open project's site with its own dev server and opens it in
 * the browser. The first start can take a while; the button stays busy
 * until the site answers or the start fails. While the server runs for
 * this project, whoever started it, the button opens it again and Stop
 * sits beside it.
 */
export function usePreviewSite() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const notify = useAppStore(s => s.notify);
  const server = usePageStore(s => s.server);
  const syncServer = usePageStore(s => s.syncServer);
  const openServer = usePageStore(s => s.openServer);
  const [starting, setStarting] = useState(false);
  const id = project?.id ?? null;
  const hasSite = Boolean(project?.siteRepo);

  // The server may have started before this view opened (an agent, the Page view).
  useEffect(() => {
    if (id && hasSite) void syncServer(id);
  }, [id, hasSite, syncServer]);

  const preview = useCallback(async () => {
    if (!id) return;
    setStarting(true);
    try {
      const url = await commands.sitePreview(id);
      notify({ status: 'done', title: 'Opened the site preview', detail: url });
      await syncServer(id);
    } catch (error) {
      notify({ status: 'failed', title: 'Could not start the site', detail: errorMessage(error) });
    } finally {
      setStarting(false);
    }
  }, [id, notify, syncServer]);

  const live = serverLive(server, id);
  const running = live && server?.state === 'running';
  return {
    id,
    hasSite,
    starting: starting || (live && !running),
    running,
    openTooltip: server
      ? `Opens ${serverHost(server) ?? 'the site'} in your browser; its dev server was ${serverOwner(server)}.`
      : '',
    preview,
    open: () => void openServer(),
  };
}
