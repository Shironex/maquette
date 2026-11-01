import { useState } from 'react';
import { readJson, writeJson } from '@/lib/storage';
import { nextStep, type NextAction } from '@/state/next-step';
import { isActive } from '@/state/providers';
import { blenderTrouble } from '@/state/toolchain';
import { useAppStore } from '@/state/use-app-store';
import { useFirstRunTips } from '@/state/use-first-run-tips';
import { useGameStore } from '@/state/use-game-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useOpenSettings } from '@/state/use-settings-target';

const DISMISSED_KEY = 'maquette.board.nextStepDismissed';

/** The one step the open project is waiting on, with where its button leads. */
export function useNextStepStrip() {
  const project = useAppStore(s => s.workspace?.project);
  const tools = useAppStore(s => s.tools);
  const setView = useAppStore(s => s.setView);
  const openSettings = useOpenSettings();
  const inbox = useInboxStore(s => s.snapshot?.items.length ?? 0);
  const openImport = useGameStore(s => s.openImport);
  const jobs = useProvidersStore(s => s.jobs);
  const openPanel = useProvidersStore(s => s.openPanel);
  const openBatch = useProvidersStore(s => s.openBatch);
  const tipsHidden = useFirstRunTips(s => s.hidden);
  const pipeline = usePipelineStore(s => (s.view?.project === project?.id ? s.view : null));
  const [dismissed, setDismissed] = useState(() => readJson(DISMISSED_KEY));
  if (!project) return null;
  const latest = pipeline?.runs[0];
  const step = nextStep({
    blender: tools.checking ? null : blenderTrouble(tools.reports.find(r => r.id === 'blender')),
    inbox,
    running: Boolean(pipeline?.active),
    tipsShowing: !tipsHidden,
    target: project.target ?? 'website',
    hasRepo: Boolean(project.siteRepo),
    assets: project.assets ?? [],
    sending: new Set(jobs.filter(j => isActive(j.state)).map(j => j.asset)),
    lastRun: latest ? { state: latest.state } : undefined,
  });
  // Before the pipeline is read a project with models would ask for a first run it may have had.
  if (!step || step.id === dismissed || (step.id === 'run' && !pipeline)) return null;
  const acts: Record<NextAction, () => void> = {
    toolchain: () => openSettings('toolchain'),
    repo: () => openSettings('project'),
    inbox: () => setView('inbox'),
    lab: () => setView('lab'),
    pipeline: () => setView('pipeline'),
    import: () => void openImport(project.id),
    send: () => {
      const [first, ...rest] = step.assets ?? [];
      if (!first) return;
      if (rest.length === 0) void openPanel(project.id, first);
      else void openBatch(project.id, [first, ...rest]);
    },
  };
  return {
    step,
    act: acts[step.action],
    dismiss: () => {
      writeJson(DISMISSED_KEY, step.id);
      setDismissed(step.id);
    },
  };
}
