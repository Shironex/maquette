import { useState } from 'react';
import type { RigRecord } from '@/ipc/client';
import { redoBlocker } from '@/state/redo';
import { useAppStore } from '@/state/use-app-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRigStore } from '@/state/use-rig-store';

/** The rig store's answers, and the project an Inbox model would be accepted into. */
export function useRigActions() {
  const rig = useRigStore();
  const project = useAppStore(s => s.workspace?.project?.id);
  return { ...rig, project };
}

/** A choice a rig doctor view keeps for itself: the sheet view, the compare mode. */
export function useChoice<T>(first: T): [T, (next: T) => void] {
  return useState<T>(first);
}

/**
 * Redo on a refused model: the send it came from, opened in the send panel
 * with the existing redo flow. The panel is where credits get approved.
 */
export function useRigRedo(record: RigRecord) {
  const openRedo = useProvidersStore(s => s.openRedo);
  const jobs = useProvidersStore(s => s.jobs);
  const items = useInboxStore(s => s.snapshot?.items);
  const { inbox, project, asset } = record.source;
  const link = inbox ? items?.find(i => i.id === inbox)?.link : null;
  const job = !inbox && project && asset ? jobs.find(j => j.asset === asset) : null;
  const from = link ?? (job ? { project: job.project, asset: job.asset, job: job.id } : null);
  return {
    blocker: from ? null : redoBlocker({ link: null }),
    open: () => {
      if (from) void openRedo(from.project, from.asset, from.job);
    },
  };
}
