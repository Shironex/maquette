import { useState } from 'react';
import type { PipelineView } from '@/ipc/client';
import { readFlag, writeFlag } from '@/lib/storage';
import { starterOffer } from '@/state/pipeline';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** "Keep mine" for an edited script, remembered per project and script. */
const keepKey = (project: string, path: string) => `maquette.starter.keep.${project}.${path}`;

/** The starters to offer, which one is updating, and Update, Replace and Keep mine. */
export function useStartersBanner(view: PipelineView) {
  const updateStarters = usePipelineStore(s => s.updateStarters);
  // The banner names its project: Update acts on this one, not the one an agent talked about.
  const projectName = useAppStore(
    s => s.workspace?.projects.find(p => p.id === view.project)?.name ?? view.project
  );
  const [working, setWorking] = useState<string | null>(null);
  const [kept, setKept] = useState<readonly string[]>([]);
  const { update, edited } = starterOffer(view.starters);
  const shown = edited.filter(p => !kept.includes(p) && !readFlag(keepKey(view.project, p)));

  const run = async (id: string, scripts: string[], replace: boolean) => {
    setWorking(id);
    await updateStarters(scripts, replace);
    setWorking(null);
  };
  const keep = (path: string) => {
    writeFlag(keepKey(view.project, path), true);
    setKept(k => [...k, path]);
  };
  return { update, shown, working, run, keep, projectName };
}
