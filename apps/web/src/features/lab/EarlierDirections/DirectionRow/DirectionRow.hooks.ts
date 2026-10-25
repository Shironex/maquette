import { useState } from 'react';
import type { Direction, LabView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { useAppStore } from '@/state/use-app-store';
import { coverImage, inProject } from '@/state/lab';
import { useLabStore } from '@/state/use-lab-store';

/** The row's cover, the projects it can move to, and the two step menu. */
export function useDirectionRow(
  view: LabView,
  d: Direction,
  menuOpen: boolean,
  onMenu: (open: boolean) => void
) {
  const selected = useLabStore(s => s.selected);
  const reuse = useLabStore(s => s.reuse);
  const move = useLabStore(s => s.moveDirection);
  const busy = useLabStore(s => s.busy);
  const projects = useAppStore(s => s.workspace?.projects) ?? [];
  const others = projects.filter(p => p.id !== view.project);
  const generating = view.jobs.some(
    j => j.direction === d.id && (j.state === 'queued' || j.state === 'running')
  );
  const cover = coverImage(d);
  const url = cover ? fileUrl(inProject(view.projectDir, cover.file)) : null;
  const current = d.id === selected;
  const [stage, setStage] = useState<'actions' | 'move'>('actions');
  const toggle = () => {
    setStage('actions');
    onMenu(!menuOpen);
  };
  return { reuse, move, busy, others, generating, url, current, stage, setStage, toggle };
}
