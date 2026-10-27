import { useEffect, useState } from 'react';
import { useShipApproval } from '../ShipView/ShipView.hooks';
import type { GameSettingsChange } from '@/ipc/client';
import { gameOf } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useGameStore } from '@/state/use-game-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useShipStore } from '@/state/use-ship-store';

/** The open project's pipeline, plus the log filter, the cache and build only switches. */
export function usePipelineView() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const budget = useAppStore(s => s.workspace?.config.budgets.sceneMb ?? 3);
  const { view, loading, error, starting, run, cancel, savePresets, saveBake, load } =
    usePipelineStore();
  const [force, setForce] = useState(false);
  const [buildOnly, setBuildOnly] = useState(false);
  const [logStep, setLogStep] = useState('all');
  const game = gameOf(project);
  const saveSettings = useGameStore(s => s.saveSettings);
  const shipWaits = useShipApproval(project?.id) !== null;
  // A game's Ship waiting for approval opens the Ship diff; another project closes it.
  // A website's Ship settings open only from their button.
  const [shipOpen, setShipOpen] = useState(false);
  const isGame = game !== null;
  useEffect(() => setShipOpen(isGame && shipWaits), [isGame, shipWaits, project?.id]);
  const loadShip = useShipStore(s => s.load);
  // The presets panel greys the codec out when the textures ship as WebP or AVIF.
  const projectId = project?.id;
  useEffect(() => {
    if (projectId && !isGame) void loadShip(projectId);
  }, [projectId, isGame, loadShip]);
  const textures = useShipStore(s =>
    s.view && s.view.project === project?.id ? s.view.settings.textures : null
  );
  const onDisk = (project?.assets ?? []).reduce(
    (sum, a) => sum + (a.versions[a.versions.length - 1]?.stats?.bytes ?? 0),
    0
  );
  const saveGame = (change: GameSettingsChange) =>
    project ? saveSettings(project.id, change) : Promise.resolve(false);
  // The log filter and the switches belong to the project they were set in.
  useEffect(() => {
    setForce(false);
    setBuildOnly(false);
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
    switches: { force, setForce, buildOnly, setBuildOnly },
    textures,
    logStep,
    setLogStep,
    game,
    onDisk,
    saveGame,
    ship: { open: shipOpen, setOpen: setShipOpen, waits: shipWaits },
  };
}
