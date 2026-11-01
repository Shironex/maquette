import { useRef, useState } from 'react';
import { sendCounts } from '@/state/sends';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { usePackStore } from '@/state/use-pack-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { missingPreviews } from '@/state/thumbs';
import { useRecentlyRemoved } from '../../RecentlyRemoved';

/** The header's quieter actions: the pack, previews, every send and the trash. */
export function useMoreMenu() {
  const project = useAppStore(s => s.workspace?.project);
  const setQueueOpen = useProvidersStore(s => s.setQueueOpen);
  const jobs = useProvidersStore(s => s.jobs);
  const renderPreviews = useGameStore(s => s.renderPreviews);
  const asking = useGameStore(s => s.rendering);
  const waiting = useGameStore(s => s.queued.length);
  const removed = useRecentlyRemoved();
  const [open, setOpen] = useState(false);
  const wrap = useRef<HTMLDivElement>(null);

  return {
    wrap,
    project,
    open,
    setOpen,
    removed,
    failed: sendCounts(jobs).failed,
    missing: missingPreviews(project),
    rendering: asking || waiting > 0,
    waiting,
    allSends: () => setQueueOpen(true),
    /** Every accepted model as one pack: the Library's pack sheet opens on it. */
    exportPack: () => {
      if (!project) return;
      useAppStore.getState().setView('library');
      void usePackStore
        .getState()
        .show({ from: 'project', project: project.id }, { project: project.name });
    },
    renderMissing: () => {
      if (project) void renderPreviews(project.id);
    },
  };
}
