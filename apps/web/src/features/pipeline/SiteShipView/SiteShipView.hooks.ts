import { useEffect } from 'react';
import type {
  ShipChoice,
  ShipSettingsChange_Deserialize as SettingsChange,
  SiteTarget,
} from '@/ipc/client';
import { homePath } from '@/lib/format';
import { useAppStore } from '@/state/use-app-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useShipStore } from '@/state/use-ship-store';

/** The website Ship view of `project`: settings, plan and size report, fresh after each run. */
export function useSiteShipView(project: string) {
  const { view, loading, saving, error, load, save } = useShipStore();
  const home = useAppStore(s => s.boot?.home);
  const name = useAppStore(s => s.workspace?.project?.name ?? project);
  // A finished run rewrites web/dist, so the plan and the sizes change.
  const runKey = usePipelineStore(s => {
    const latest = s.view?.runs[0];
    return latest ? `${latest.id} ${latest.state}` : '';
  });
  useEffect(() => {
    void load(project);
  }, [project, runKey, load]);

  const ours = view?.project === project ? view : null;
  const saveSettings = (settings: SettingsChange) => save(project, { settings });
  const saveTarget = (target: SiteTarget) => save(project, { target });
  /** Sets one asset's choice, keeping its file name. */
  const setChoice = (asset: string, choice: ShipChoice) =>
    saveSettings({ assets: { [asset]: { choice, file: fileOf(asset) } } });
  /** Sets one asset's file name, keeping its choice; empty goes back to the default. */
  const setFile = (asset: string, file: string) =>
    saveSettings({ assets: { [asset]: { choice: choiceOf(asset), file: file.trim() || null } } });
  const entry = (asset: string) => ours?.settings.assets[asset];
  const fileOf = (asset: string) => entry(asset)?.file ?? null;
  const choiceOf = (asset: string): ShipChoice => entry(asset)?.choice ?? 'auto';
  const repo = ours?.plan.target.repo ? homePath(ours.plan.target.repo, home) : null;
  return {
    view: ours,
    loading,
    saving,
    error,
    name,
    repo,
    reload: () => load(project),
    saveSettings,
    saveTarget,
    setChoice,
    setFile,
    fileOf,
  };
}

/** "1.9 MB" style numbers for the plan, or a dash when not measured. */
export function sizeOrDash(bytes: number | null, format: (n: number) => string): string {
  return bytes === null || bytes === 0 ? '·' : format(bytes);
}

/** "new", "replaces 1.2 MB" or "same bytes": what Ship does to the site file. */
export function changeWords(change: string, before: string | null): string {
  if (change === 'new') return 'new to the site';
  if (change === 'unchanged') return 'same bytes, nothing to do';
  if (change === 'hand-made') return before ? `site file ${before}, made by hand` : 'made by hand';
  return before ? `replaces ${before}` : 'replaces the site file';
}
