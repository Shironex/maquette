import { useState } from 'react';
import type { MenuItem } from '@/features/common/Menu';
import { serverOwner } from '@/state/page';
import { useAppStore } from '@/state/use-app-store';
import { usePageStore } from '@/state/use-page-store';
import { useProjectsStore } from '@/state/use-projects-store';

/** The status bar's dev server menu: open the site, show it in the Page view, stop it. */
export function useSiteServerItem() {
  const server = usePageStore(s => s.server);
  const openServer = usePageStore(s => s.openServer);
  const stopServer = usePageStore(s => s.stopServer);
  const openProject = useAppStore(s => s.workspace?.project ?? null);
  const current = openProject?.id ?? null;
  const setView = useAppStore(s => s.setView);
  const projects = useProjectsStore(s => s.list);
  const [open, setOpen] = useState(false);
  const running = server?.state === 'running';
  const failed = server?.state === 'failed';
  const here = server?.project === current;
  const name =
    projects.find(p => p.id === server?.project)?.name ??
    (here ? openProject?.name : null) ??
    server?.project ??
    'the site';
  const items: MenuItem[] = [
    {
      id: 'open',
      label: 'Open in browser',
      icon: 'arrow-square-out',
      disabled: !running,
      hint: running ? undefined : 'Opens once the site answers',
      onSelect: () => void openServer(),
    },
    {
      id: 'page',
      label: 'Show in the Page view',
      icon: 'browser',
      disabled: !here,
      hint: here ? undefined : `It serves ${name}; switch to that project first`,
      onSelect: () => setView('page'),
    },
    {
      id: 'stop',
      section: 'stop',
      label: failed ? 'Dismiss' : server?.external ? 'Forget it' : 'Stop the dev server',
      icon: failed ? 'x' : 'stop',
      tone: failed ? undefined : 'danger',
      onSelect: () => void stopServer(),
    },
  ];
  return {
    open,
    toggle: () => setOpen(o => !o),
    close: () => setOpen(false),
    items,
    detail: server ? `${name}, ${serverOwner(server)}` : '',
    command: server?.command ?? null,
  };
}
