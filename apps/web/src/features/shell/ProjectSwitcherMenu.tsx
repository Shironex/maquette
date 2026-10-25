import type { RefObject } from 'react';
import { ProjectMenu, cn } from '@maquette/ui';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/use-app-store';
import { activityLabel, projectMeta, switchNote } from '@/state/projects';
import { useProjectsStore } from '@/state/use-projects-store';
import { projectThumb } from '@/features/common/projectArt';

/** The switcher's popover, under the switcher or beside it on a collapsed rail. */
export function ProjectSwitcherMenu({
  anchorRef,
  collapsed,
  shortcut,
}: {
  anchorRef: RefObject<HTMLButtonElement | null>;
  collapsed: boolean;
  shortcut: string;
}) {
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
        siteRepo: p.siteRepo,
        createdAt: p.createdAt,
        assetCount: p.assets?.length ?? 0,
        dir: '',
        thumb: null,
        activity: { run: null, concepts: 0, sends: 0 },
      }));
  const dismiss = () => {
    close();
    anchorRef.current?.focus();
  };
  return (
    <ProjectMenu
      anchorRef={anchorRef}
      shortcut={shortcut}
      className={cn(
        'absolute z-50',
        collapsed ? 'top-0 left-[calc(100%+14px)]' : 'top-[calc(100%+6px)] left-0'
      )}
      projects={rows.map(p => ({
        id: p.id,
        name: p.name,
        meta: projectMeta(p, now),
        thumb: projectThumb(p, p),
        current: p.id === currentId,
        live: activityLabel(p.activity) ?? undefined,
      }))}
      note={switchNote(rows.find(p => p.id === currentId)) ?? undefined}
      actions={[
        { id: 'new', label: 'New project', icon: 'plus', onSelect: () => openDialog('template') },
        {
          id: 'import',
          label: 'Import folder',
          icon: 'folder-open',
          onSelect: () => openDialog('import'),
        },
      ]}
      onSelect={id => {
        void switchTo(id);
        anchorRef.current?.focus();
      }}
      onClose={dismiss}
    />
  );
}
