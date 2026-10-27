import { useState } from 'react';
import { serverHost, serverLive, serverOwner } from '@/state/page';
import { usePageStore } from '@/state/use-page-store';

/** What Stop says and does: the shared dev server, while it serves `project`. */
export function useStopServer(project: string | null) {
  const server = usePageStore(s => s.server);
  const stopServer = usePageStore(s => s.stopServer);
  const [stopping, setStopping] = useState(false);
  const live = project !== null && serverLive(server, project);
  const where = serverHost(server) ?? 'its port';
  const tooltip = !server
    ? ''
    : server.external
      ? `Forgets the site on ${where}. Maquette did not start it, so it keeps running.`
      : `Stops the site's dev server on ${where}, ${serverOwner(server)}.`;
  // The icon's tooltip names the action, so its second line only says where and who.
  const detail = !server
    ? ''
    : server.external
      ? `On ${where}. Maquette did not start it, so Stop only forgets it.`
      : `On ${where}, ${serverOwner(server)}.`;
  const stop = async () => {
    setStopping(true);
    await stopServer();
    setStopping(false);
  };
  return { live, stopping, tooltip, detail, stop };
}
