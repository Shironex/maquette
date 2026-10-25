import { useEffect, useState } from 'react';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The open project's pipeline, plus the log filter and the cache switch. */
export function usePipelineView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const budget = useAppStore(s => s.workspace?.config.budgets.sceneMb ?? 3);
  const { view, loading, error, starting, run, cancel, savePresets, saveBake, load } =
    usePipelineStore();
  const [force, setForce] = useState(false);
  const [logStep, setLogStep] = useState('all');
  // The log filter and the cache switch belong to the project they were set in.
  useEffect(() => {
    setForce(false);
    setLogStep('all');
  }, [project?.id]);
  return {
    project,
    budget,
    view,
    loading,
    error,
    starting,
    run,
    cancel,
    savePresets,
    saveBake,
    load,
    force,
    setForce,
    logStep,
    setLogStep,
  };
}
