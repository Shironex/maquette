import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import type { CompareSide, CompressPreset, InspectModel, InspectView } from '@/ipc/client';
import { fileUrl } from '@/ipc/files';
import { projectFile } from '@/state/board';
import {
  modelForBudget,
  pickSides,
  sideModels,
  stopsOf,
  vec3,
  type SidePick,
} from '@/state/inspector';
import { isGame } from '@/state/game';
import { useAppStore } from '@/state/use-app-store';
import { useInspectorStore } from '@/state/use-inspector-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import type { CompareCanvasHandlers, CompareModel } from '../CompareCanvas';

/**
 * The comparison picked and the models on each side, with the scene's
 * camera stops. Starts at the first stop, as the site does.
 */
export function useCompareSides(view: InspectView) {
  const comparison = useInspectorStore(s => s.comparison);
  const leftId = useInspectorStore(s => s.left);
  const rightId = useInspectorStore(s => s.right);
  const stop = useInspectorStore(s => s.stop);
  const flyTo = useInspectorStore(s => s.flyTo);
  const pick = pickSides(view, comparison, leftId, rightId);
  const kind = pick?.comparison.kind ?? 'scene';
  const stops = useMemo(() => (kind === 'scene' ? stopsOf(view) : []), [kind, view]);

  const toCompare = useCallback(
    (side: CompareSide | null): CompareModel[] =>
      side
        ? side.members.flatMap(m => {
            const model = view.models.find(x => x.id === m.model);
            if (!model || model.error) return [];
            const url = fileUrl(model.file);
            if (!url) return [];
            return [{ id: m.model, url, position: vec3(m.position), asset: model.tweakAsset }];
          })
        : [],
    [view]
  );
  const left = useMemo(() => toCompare(pick?.left ?? null), [toCompare, pick?.left]);
  const right = useMemo(() => toCompare(pick?.right ?? null), [toCompare, pick?.right]);

  const started = useRef(false);
  useEffect(() => {
    if (started.current || stops.length === 0 || stop) return;
    started.current = true;
    flyTo(0);
  }, [stops.length, stop, flyTo]);

  return { pick, kind, stops, left, right };
}

/**
 * How the compare canvas is doing: loaded, failed, its stop renders and
 * lightmap count, all fresh for each comparison; and its callbacks.
 */
export function useCompareStatus(pick: SidePick | null) {
  const sample = useInspectorStore(s => s.sample);
  const select = useInspectorStore(s => s.select);
  const leaveStop = useInspectorStore(s => s.leaveStop);
  const [shots, setShots] = useState<Record<number, string>>({});
  const [lightmaps, setLightmaps] = useState<number | null>(null);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useEffect(() => {
    setShots({});
    setReady(false);
    setLoadError(null);
  }, [pick?.comparison.id, pick?.right.id]);

  const handlers: CompareCanvasHandlers = {
    onSample: sample,
    onPick: id => {
      if (id) select(id);
    },
    onStopShots: setShots,
    onUserMove: leaveStop,
    onLightmaps: setLightmaps,
    onError: setLoadError,
    onReady: () => setReady(true),
  };
  return {
    shots,
    lightmaps,
    clearLightmaps: () => setLightmaps(null),
    loadError,
    ready,
    handlers,
  };
}

/**
 * The model the panel shows: the one selected, else the right side's
 * character, else its first; with its thumbnail and export.
 */
export function useSelectedModel(view: InspectView, pick: SidePick | null) {
  const workspace = useAppStore(s => s.workspace);
  const selected = useInspectorStore(s => s.selected);
  const select = useInspectorStore(s => s.select);
  const exporting = useInspectorStore(s => s.exporting);
  const exportGlb = useInspectorStore(s => s.exportGlb);
  const rightModels = sideModels(view, pick?.right ?? null);
  const model =
    view.models.find(m => m.id === selected) ??
    rightModels.find(m => m.model === 'character') ??
    rightModels[0] ??
    null;
  const asset = model?.asset ? workspace?.project?.assets?.find(a => a.id === model.asset) : null;
  const thumbPath = asset?.thumbnail && workspace ? projectFile(workspace, asset.thumbnail) : null;
  return {
    model,
    thumb: fileUrl(thumbPath),
    onPickBudget: (stem: string) => {
      const found = modelForBudget(view, pick?.right ?? null, stem);
      if (found) select(found.id);
    },
    onExport: (m: InspectModel) => void exportGlb(m.model, m.label),
    exporting: Boolean(model && exporting === model.model),
  };
}

/**
 * The page's own controls: the view mode, the turntable, the frame time
 * samples, running the pipeline and saving a model's preset.
 */
export function useInspectorPage(view: InspectView) {
  const setView = useAppStore(s => s.setView);
  const game = useAppStore(s => isGame(s.workspace?.project));
  const pipeline = usePipelineStore();
  const mode = useInspectorStore(s => s.mode);
  const setMode = useInspectorStore(s => s.setMode);
  const samples = useInspectorStore(s => s.samples);
  const load = useInspectorStore(s => s.load);
  const [turntable, setTurntable] = useState(false);

  const savePreset = pipeline.view
    ? (row: CompressPreset) => {
        const rows = pipeline.view?.def.presets ?? [];
        const next = rows.some(r => r.model === row.model)
          ? rows.map(r => (r.model === row.model ? { ...row, lock: r.lock } : r))
          : [...rows, row];
        void pipeline.savePresets(next).then(ok => {
          if (!ok) return;
          void load(view.project);
        });
      }
    : null;
  const running = Boolean(pipeline.view?.active);

  return {
    setView,
    /** What the empty page asks for: a game has no scene to export. */
    emptyBody: game
      ? 'Import the models from the game repo, or accept a model from the Inbox. Both show up here to compare.'
      : 'Run the pipeline to export the scene, or accept a model from the Inbox. Both show up here to compare.',
    mode,
    setMode,
    samples,
    turntable,
    toggleTurntable: () => setTurntable(t => !t),
    savePreset,
    runDisabled: running || pipeline.starting || !pipeline.view,
    run: () => void pipeline.run({}),
  };
}
