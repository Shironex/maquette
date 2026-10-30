import { useState } from 'react';
import type { LibraryItem } from '@/ipc/client';
import { clock } from '@/lib/clock';
import { latestOf, useBlocked, useNote } from '@/state/library';
import { useAppStore } from '@/state/use-app-store';
import { useLibraryStore } from '@/state/use-library-store';
import { useLibraryQuickView } from '@/state/use-quick-view-store';

/** The project and version Use in project copies, the open project first and the newest version. */
export function useLibraryDetail(item: LibraryItem) {
  const workspace = useAppStore(s => s.workspace);
  const { use, openUpdate, busy } = useLibraryStore();
  const quickView = useLibraryQuickView()(item);
  const projects = workspace?.projects ?? [];
  const [projectId, setProjectId] = useState(workspace?.project?.id ?? projects[0]?.id ?? '');
  const newest = latestOf(item)?.version ?? 1;
  const [version, setVersion] = useState(newest);
  const project = projects.find(p => p.id === projectId) ?? null;
  const blocked = useBlocked(item, project);
  const spent = item.spent ?? 0;
  const saved = item.saved ?? 0;
  const reuses = item.uses.filter(
    u => !item.entry.versions.some(v => v.from.project === u.project && v.from.asset === u.asset)
  ).length;

  return {
    now: clock.nowMs(),
    quickView,
    projects,
    project,
    projectId,
    setProjectId,
    version,
    setVersion,
    newest,
    blocked,
    note: useNote(item, project),
    busy,
    useIt: () => {
      if (project) void use(item.entry.id, version === newest ? null : version, project.id);
    },
    update: (target: { project: string; asset: string }) => void openUpdate(target),
    credits: [
      { label: 'Spent to make it', value: String(spent) },
      {
        label: 'Saved by reusing it',
        value: reuses ? `${saved} (${reuses} ${reuses === 1 ? 'reuse' : 'reuses'})` : '0',
      },
    ],
  };
}
