import { useEffect, useMemo, useReducer } from 'react';
import { readJson, writeJson } from '@/lib/storage';
import {
  forgetDismissed,
  nextSteps,
  visibleStep,
  type Dismissal,
  type NextAction,
} from '@/state/next-step';
import { isActive } from '@/state/providers';
import { blenderTrouble } from '@/state/toolchain';
import { useAppStore } from '@/state/use-app-store';
import { useFirstRunTips } from '@/state/use-first-run-tips';
import { useGameStore } from '@/state/use-game-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';
import { useOpenSettings } from '@/state/use-settings-target';
import { useToolRecheckOnFocus } from '@/state/useToolRecheckOnFocus';

const dismissedKey = (project: string) => `maquette.board.nextStepDismissed.${project}`;

function readDismissed(project: string): readonly Dismissal[] {
  const stored = readJson(dismissedKey(project));
  return Array.isArray(stored) ? (stored as Dismissal[]) : [];
}

/** The one step the open project is waiting on, with where its button leads. */
export function useNextStepStrip() {
  const project = useAppStore(s => s.workspace?.project);
  const tools = useAppStore(s => s.tools);
  const setView = useAppStore(s => s.setView);
  const openSettings = useOpenSettings();
  const inboxSnapshot = useInboxStore(s => s.snapshot);
  const inbox = inboxSnapshot?.items.length ?? 0;
  const openImport = useGameStore(s => s.openImport);
  const jobs = useProvidersStore(s => s.jobs);
  const jobsRead = useProvidersStore(s => s.jobsReadFor === project?.id);
  const openPanel = useProvidersStore(s => s.openPanel);
  const openBatch = useProvidersStore(s => s.openBatch);
  const openRefs = useRefsStore(s => s.open);
  const tipsHidden = useFirstRunTips(s => s.hidden);
  const pipeline = usePipelineStore(s => (s.view?.project === project?.id ? s.view : null));
  const [, refresh] = useReducer((n: number) => n + 1, 0);
  const trouble = blenderTrouble(tools.reports.find(r => r.id === 'blender'));
  // Mounted on the Board, so a Blender put right outside the app clears the strip here too.
  useToolRecheckOnFocus(trouble !== null);
  const latest = pipeline?.runs[0];
  const running = Boolean(pipeline?.active);
  const steps = useMemo(
    () =>
      project
        ? nextSteps({
            blender: tools.checking ? null : trouble,
            inbox,
            running,
            tipsShowing: !tipsHidden,
            target: project.target ?? 'website',
            hasRepo: Boolean(project.siteRepo),
            imported: project.template === 'imported',
            assets: project.assets ?? [],
            sending: new Set(jobs.filter(j => isActive(j.state)).map(j => j.asset)),
            lastRun: latest ? { id: latest.id, state: latest.state } : undefined,
          })
        : [],
    [project, tools.checking, trouble, inbox, running, tipsHidden, jobs, latest]
  );
  // Only what has really been read can say a step is gone, not a check or a load under way.
  const settled =
    !tools.checking &&
    tools.checkedAt !== null &&
    pipeline !== null &&
    !running &&
    jobsRead &&
    inboxSnapshot !== null;
  const projectId = project?.id;
  useEffect(() => {
    if (!settled || !projectId) return;
    const dismissed = readDismissed(projectId);
    const kept = forgetDismissed(dismissed, steps);
    if (kept === dismissed) return;
    writeJson(dismissedKey(projectId), kept);
    refresh();
  }, [settled, projectId, steps]);
  if (!project) return null;
  const dismissed = readDismissed(project.id);
  const step = visibleStep(
    // Before the pipeline is read a project with models would ask for a first run it may have had.
    pipeline ? steps : steps.filter(s => s.id !== 'run'),
    dismissed
  );
  if (!step) return null;
  const acts: Record<NextAction, () => void> = {
    toolchain: () => openSettings('toolchain'),
    repo: () => openSettings('project'),
    inbox: () => setView('inbox'),
    lab: () => setView('lab'),
    pipeline: () => setView('pipeline'),
    import: () => void openImport(project.id),
    refs: () => {
      const first = step.assets?.[0];
      if (first) void openRefs(project.id, first);
    },
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
      writeJson(dismissedKey(project.id), [...dismissed, { key: step.key, count: step.count }]);
      refresh();
    },
  };
}
