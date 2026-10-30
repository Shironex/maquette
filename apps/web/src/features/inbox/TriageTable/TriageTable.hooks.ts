import { useState } from 'react';
import type { InboxItem } from '@/ipc/client';
import { goesTo } from '@/state/inbox';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useTriageStore } from '@/state/use-triage-store';
import type { TriageGroup } from '@/state/triage';

/** Rows a group shows before "N more". */
export const GROUP_ROWS = 8;

/** The table's opened groups, where each model goes and opening one by one. */
export function useTriageTable() {
  const workspace = useAppStore(s => s.workspace);
  const select = useInboxStore(s => s.select);
  const setLayout = useTriageStore(s => s.setLayout);
  const [opened, setOpened] = useState<ReadonlySet<TriageGroup>>(new Set());
  const current = workspace?.project ?? null;
  const projects = workspace?.projects ?? [];
  return {
    isOpen: (group: TriageGroup) => opened.has(group),
    open: (group: TriageGroup) => setOpened(s => new Set(s).add(group)),
    /** "Ice crystals", with the project named when it is not the open one; "New asset" without a goal. */
    goesToCell: (item: InboxItem): { asset: string; project: string | null } => {
      const goal = goesTo(item);
      if (!goal) return { asset: 'New asset', project: current?.name ?? null };
      const other =
        goal.project === current?.id
          ? null
          : (projects.find(p => p.id === goal.project)?.name ?? goal.project);
      return { asset: goal.assetName, project: other };
    },
    /** Shows the model in the one by one layout, with its preview and checks. */
    openOne: (id: string) => {
      select(id);
      setLayout('one');
    },
  };
}
