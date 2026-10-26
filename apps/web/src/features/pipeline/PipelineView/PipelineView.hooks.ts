import { useEffect, useState } from 'react';
import { useShipApproval } from '../ShipView/ShipView.hooks';
import type { GameSettingsChange } from '@/ipc/client';
import { gameOf } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { usePipelineStore } from '@/state/use-pipeline-store';

/** The open project's pipeline, plus the log filter and the cache switch. */
export function usePipelineView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const budget = useAppStore(s => s.workspace?.config.budgets.sceneMb ?? 3);
  const { view, loading, error, starting, run, cancel, savePresets, saveBake, load } =
    usePipelineStore();
  const [force, setForce] = useState(false);
  const [logStep, setLogStep] = useState('all');
  const game = gameOf(project);
  const saveSettings = useGameStore(s => s.saveSettings);
  const shipWaits = useShipApproval(project?.id) !== null;
  // A game's Ship waiting for approval opens the Ship diff; another project closes it.
  const [shipOpen, setShipOpen] = useState(false);
  useEffect(() => setShipOpen(shipWaits), [shipWaits, project?.id]);
  const onDisk = (project?.assets ?? []).reduce(
    (sum, a) => sum + (a.versions[a.versions.length - 1]?.stats?.bytes ?? 0),
    0
  );
  const saveGame = (change: GameSettingsChange) =>
    project ? saveSettings(project.id, change) : Promise.resolve(false);
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
    game,
    onDisk,
    saveGame,
    ship: { open: shipOpen, setOpen: setShipOpen, waits: shipWaits },
  };
}
