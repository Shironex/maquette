import type {
  CompressPreset,
  PipelineView,
  ProjectManifest,
  SceneLayout,
  StarterStatus,
  WorkspaceConfig,
} from '../client';
import { DEFAULT_ROOT } from './fixtures';
import { inspectView } from './inspect-fixtures';
import { expand, fail, type Args } from './backend-options';
import type { createFakeLayout } from './layout';
import type { createFakeRunner, RunStore } from './runner';

interface PipelineContext {
  project: (id: string) => ProjectManifest;
  config: () => WorkspaceConfig;
  root: () => string | null;
  layout: ReturnType<typeof createFakeLayout>;
  runner: ReturnType<typeof createFakeRunner>;
  store: RunStore;
  presets: Map<string, CompressPreset[]>;
  starters: Map<string, StarterStatus[]>;
  view: (id: string) => PipelineView;
  startersOf: (id: string) => StarterStatus[];
  activeRun: (id: string) => string | null;
}

/** The layout, pipeline and inspect commands. */
export function createPipelineHandlers({
  project,
  config,
  root,
  layout,
  runner,
  store,
  presets,
  starters,
  view,
  startersOf,
  activeRun,
}: PipelineContext): Record<string, (args: Args) => unknown> {
  return {
    layout_get: ({ project: id }) => layout.view(String(id)),
    layout_save: ({ project: id, layout: next }) => {
      project(String(id));
      return layout.save(String(id), next as SceneLayout);
    },
    pipeline_get: ({ project: id }) => view(String(id)),
    pipeline_run: ({ project: id, from, force }) => {
      const def = view(String(id)).def;
      return runner.submit(String(id), def, (from as string | null) ?? null, Boolean(force));
    },
    pipeline_cancel: ({ runId }) => runner.cancel(String(runId)),
    pipeline_log: ({ number }) => store.logs.get(Number(number)) ?? [],
    pipeline_update_starters: ({ project: id, scripts, replace }) => {
      const pid = String(id);
      if (activeRun(pid))
        fail('invalid', 'The pipeline is running. Update the scripts once the run is done.');
      const asked = new Set(scripts as string[]);
      starters.set(
        pid,
        startersOf(pid).map(s => {
          if (!asked.has(s.path) || s.state === 'current') return s;
          if (s.state === 'edited' && !replace) return s;
          return {
            ...s,
            state: 'current',
            backup: s.state === 'edited' ? `${s.path}.bak` : null,
          };
        })
      );
      return view(pid);
    },
    pipeline_save_presets: ({ project: id, presets: rows }) => {
      const next = rows as CompressPreset[];
      const locked = view(String(id)).def.presets.filter(p => p.lock);
      // Field by field, as the core compares; a missing data map size is none.
      const plain = (p: CompressPreset) =>
        JSON.stringify([
          p.label,
          p.textureSize,
          p.dataTextureSize ?? null,
          p.codec,
          p.mapCodecs ?? null,
          p.meshopt,
          p.lock,
        ]);
      for (const old of locked) {
        const same = next.find(p => p.model === old.model);
        if (!same || plain(same) !== plain(old)) fail('invalid', old.lock ?? 'That row is locked.');
      }
      presets.set(String(id), next);
      return view(String(id));
    },
    inspect_project: ({ project: id }) => {
      const p = project(String(id));
      const v = view(p.id);
      // Only Moon Island has shipped models in mock mode.
      if (p.id !== 'moon-island') {
        return {
          project: p.id,
          models: [],
          comparisons: [],
          stops: { source: 'none', file: null, stops: [] },
          budgets: [],
          limits: {
            sceneMb: config().budgets.sceneMb ?? 3,
            trisPerModel: config().budgets.trisPerModel,
            firstViewMb: config().budgets.firstViewMb ?? 3,
          },
          scene: { bytes: 0, triangles: 0, models: 0 },
        };
      }
      return inspectView(
        p.id,
        `${root() ?? expand(DEFAULT_ROOT)}/projects/${p.id}`,
        v.def.presets,
        v.defaults,
        config().budgets.sceneMb ?? 3,
        config().budgets.trisPerModel
      );
    },
    inspect_export_glb: ({ project: id, model }) => {
      project(String(id));
      return { model: String(model), file: `web/raw/${String(model)}.glb`, runNumber: 27 };
    },
  };
}
