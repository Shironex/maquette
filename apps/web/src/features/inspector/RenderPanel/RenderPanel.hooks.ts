import { useEffect, useMemo } from 'react';
import type { PackSource, RenderFile, RenderProblem, RenderStyle } from '@/ipc/client';
import { isUsable } from '@/state/toolchain';
import { useAppStore } from '@/state/use-app-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRenderStore } from '@/state/use-render-store';
import { useRenderTabStore } from '@/state/use-render-tab-store';

const QUIET_MS = 350;

export interface FoundProblem {
  problem: RenderProblem;
  style: RenderStyle;
  bytes: number | null;
}

const MISSING_BLENDER: RenderProblem = {
  kind: 'missing-tool',
  message:
    'Render styles use headless Blender. Set its path in Settings, Toolchain, then render again.',
  webpBytes: null,
};

/** The problems of the newest render that holds this asset, newest style first. */
function problemsOf(files: readonly RenderFile[]): FoundProblem[] {
  return files.flatMap(f =>
    f.problem ? [{ problem: f.problem, style: f.style, bytes: f.bytes }] : []
  );
}

/**
 * The Render tab: which asset, the preview of its style at the settings on
 * show, asked again a moment after a change, and what went wrong before.
 */
export function useRenderPanel() {
  const project = useAppStore(s => s.workspace?.project ?? null);
  const reports = useAppStore(s => s.tools.reports);
  const projectId = project?.id ?? null;
  const assets = useMemo(
    () => (project?.assets ?? []).filter(a => a.kind !== 'scene'),
    [project?.assets]
  );
  const picked = useRenderTabStore(s => s.picked);
  const style = useRenderTabStore(s => s.style);
  const settings = useRenderTabStore(s => s.settings);
  const pick = useRenderTabStore(s => s.pick);
  const load = useRenderTabStore(s => s.load);
  const recent = useRenderStore(s => s.recent);
  const withModel = assets.filter(a => a.versions.length > 0);
  const asset = assets.find(a => a.id === picked) ?? withModel[0] ?? assets[0] ?? null;
  const assetId = asset?.id ?? null;
  const hasModel = (asset?.versions.length ?? 0) > 0;
  const source: PackSource | null = useMemo(
    () => (projectId ? { from: 'project', project: projectId } : null),
    [projectId]
  );

  useEffect(() => {
    void useRenderStore.getState().loadRecent();
  }, [projectId]);

  useEffect(() => {
    if (!source || !assetId || !hasModel) return;
    const timer = setTimeout(() => void load(source, assetId), QUIET_MS);
    return () => clearTimeout(timer);
  }, [source, assetId, hasModel, style, settings, load]);

  const blender = reports.find(r => r.id === 'blender');
  const blenderMissing = blender !== undefined && !isUsable(blender);
  const items = recent.flatMap(r => r.items);
  const item =
    items.find(i => i.target.key === assetId) ??
    (hasModel ? undefined : items.find(i => i.files.some(f => f.problem)));
  const problems: FoundProblem[] = [
    ...(blenderMissing ? [{ problem: MISSING_BLENDER, style: 'clay' as const, bytes: null }] : []),
    ...problemsOf(item?.files ?? []),
  ];
  return {
    source,
    assets,
    asset,
    hasModel,
    pick,
    problems,
    problemName: item?.target.name ?? asset?.name ?? '',
    pickWithModel: () => {
      const first = withModel[0];
      if (first) pick(first.id);
    },
    sendTo3d: () => {
      if (projectId && assetId) void useProvidersStore.getState().openPanel(projectId, assetId);
    },
    openToolchain: useRenderStore.getState().openToolchain,
  };
}
