import type { RefObject } from 'react';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/use-app-store';
import { activityLabel, projectMeta, switchNote } from '@/state/projects';
import { useProjectsStore } from '@/state/use-projects-store';
import { projectThumb } from '@/features/common/projectArt';

/** The menu's rows, note and actions, or null while it is closed. */
export function useProjectSwitcherMenu(anchorRef: RefObject<HTMLButtonElement | null>) {
  const open = useProjectsStore(s => s.menuOpen);
  const list = useProjectsStore(s => s.list);
  const close = useProjectsStore(s => s.closeMenu);
  const switchTo = useProjectsStore(s => s.switchTo);
  const openDialog = useProjectsStore(s => s.openDialog);
  const workspace = useAppStore(s => s.workspace);
  const now = useNow(60_000);
  if (!open || !workspace) return null;
  const currentId = workspace.project?.id;
  // Until the list with pictures and activity arrives, the snapshot's projects show.
  const rows = list.length
    ? list
    : workspace.projects.map(p => ({
        id: p.id,
        name: p.name,
        template: p.template,
        target: p.target ?? 'website',
        siteRepo: p.siteRepo,
        createdAt: p.createdAt,
        assetCount: p.assets?.length ?? 0,
        dir: '',
        thumb: null,
        activity: { run: null, concepts: 0, sends: 0 },
      }));
  return {
    projects: rows.map(p => ({
      id: p.id,
      name: p.name,
      meta: projectMeta(p, now),
      thumb: projectThumb(p, p),
      current: p.id === currentId,
      live: activityLabel(p.activity) ?? undefined,
    })),
    note: switchNote(rows.find(p => p.id === currentId)) ?? undefined,
    openDialog,
    select: (id: string) => {
      void switchTo(id);
      anchorRef.current?.focus();
    },
    dismiss: () => {
      close();
      anchorRef.current?.focus();
    },
  };
}
