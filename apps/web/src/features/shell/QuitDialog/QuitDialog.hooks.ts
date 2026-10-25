import { useEffect } from 'react';
import { commands } from '@/ipc/client';
import { events } from '@/ipc/generated/bindings';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { unappliedProjects, useQuitStore } from '@/state/use-quit-store';

/**
 * Tells the shell whether presets wait to be applied, so a close only needs
 * the web view's answer then, and opens the question when the shell asks.
 */
export function useQuitGuard() {
  const waiting = usePipelineStore(s => unappliedProjects(s.presetEdits).length > 0);
  const ask = useQuitStore(s => s.ask);
  useEffect(() => {
    void commands.quitSetUnapplied(waiting).catch(() => undefined);
  }, [waiting]);
  useEffect(() => {
    const stop = events.quitRequested.listen(ask);
    return () => void stop.then(fn => fn()).catch(() => undefined);
  }, [ask]);
}

/** The question's state and answers, and the projects whose presets wait. */
export function useQuitDialog() {
  const { asking, applying, error } = useQuitStore();
  const cancel = useQuitStore(s => s.cancel);
  const applyAndQuit = useQuitStore(s => s.applyAndQuit);
  const quitWithout = useQuitStore(s => s.quitWithoutApplying);
  const edits = usePipelineStore(s => s.presetEdits);
  const projects = useAppStore(s => s.workspace?.projects);
  const names = unappliedProjects(edits).map(id => projects?.find(p => p.id === id)?.name ?? id);
  return { asking, applying, error, cancel, applyAndQuit, quitWithout, names };
}
