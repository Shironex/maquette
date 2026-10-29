/**
 * The game commands in mock mode, the game pipeline and the `?game=`
 * states:
 * - `crypt`: Crypt game is open with 15 imported models (3 over budget,
 *   fixed by fix-ups) and 3 made in Maquette; run 7 waits for approval
 *   before Ship.
 * - `import`: Crypt game is open with nothing imported yet.
 * - `maze`: Maze game, an .obj game, is open with 35 imported models.
 * - `hero`: Crypt game with its rigged heroine imported too: 115.7k
 *   triangles, 119 bones and 187 clips, no previews rendered yet.
 */
import type {
  Approval,
  GameScan,
  GameSettings,
  GameSettingsChange,
  ImportResult,
  PipelineView,
  ProjectManifest,
  ShipFile,
  ShipPlan,
  WorkspaceConfig,
} from '../client';
import { clock } from '@/lib/clock';
import { autoFixes, budgetFor, checkAsset, fileOf, gameOf, isOver } from '@/state/game';
import { fail, type Args, type BackendOptions } from './backend-options';
import {
  HERO_MODEL,
  assetIdOf,
  gameProject,
  importedAsset,
  inspectGameRepo,
  repoModels,
  scanned,
} from './game-fixtures';
import { createPreviewHandlers } from './create-preview-handlers';
import type { RunStore } from './runner';
import { gamePipeline, gameRuns } from './game-pipeline';
import { BAKE_DEFAULT } from './bake';

const GAME_STARTERS = [
  'tools/build_models.py',
  'tools/fixups.py',
  'tools/bake_light.py',
  'tools/export_game.py',
  'tools/game_export.mjs',
  'tools/ship_game.mjs',
];

export interface GameContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  store: RunStore;
  state: string | null;
  files: BackendOptions['files'];
}

/** The folders after a change: a folder sets one, null or '' clears it. */
function withFolders(
  current: Record<string, string>,
  change: Partial<Record<string, string | null>>
): Record<string, string> {
  const next = { ...current };
  for (const [asset, folder] of Object.entries(change)) {
    if (folder) next[asset] = folder.replace(/\/+$/, '');
    else delete next[asset];
  }
  return next;
}

/** The old file of a moved imported asset, which Ship removes. */
function removedFile(
  path: string,
  bytes: number | null,
  stats: { triangles: number } | null | undefined
): ShipFile {
  return {
    path,
    change: 'removed',
    bytes: 0,
    beforeBytes: bytes,
    triangles: null,
    beforeTriangles: stats?.triangles ?? null,
    asset: null,
    blocked: false,
    imported: false,
  };
}

export function createGameHandlers({ project, saveProject, store, state, files }: GameContext) {
  const previews = createPreviewHandlers({ project, saveProject, files });
  /** Files Ship wrote before, per project, so a second Ship finds them unchanged. */
  const shipped = new Set<string>();
  const waiting = new Map<string, string>();

  function settings(id: string): GameSettings {
    const found = gameOf(project(id));
    if (!found) fail('invalid', `${project(id).name} is not a game project.`);
    return found;
  }

  function seed(): ProjectManifest[] {
    if (!state || !['crypt', 'import', 'maze', 'hero'].includes(state)) return [];
    const p = gameProject(state === 'maze' ? 'maze' : 'crypt', state !== 'import');
    if (state === 'hero') {
      p.assets = [importedAsset(HERO_MODEL, true), ...(p.assets ?? [])];
      p.game!.models[assetIdOf(HERO_MODEL.file)] = { file: HERO_MODEL.file, importedVersion: 1 };
    }
    if (state === 'crypt') {
      // Three more made in Maquette, exported and new to the game.
      const made = ['candle-cluster', 'grave-marker', 'spider'].map(id => ({
        ...importedAsset(
          {
            file: `props/${id.replaceAll('-', '_')}.glb`,
            triangles: 3_400,
            textures: 2,
            maxTexture: 1024,
            mb: 1.2,
          },
          false
        ),
        stage: 'web' as const,
      }));
      p.assets = [...(p.assets ?? []), ...made];
      const run = gameRuns(p.id, gamePipeline(p.game!), clock.nowMs());
      store.runs = [...run, ...store.runs];
      waiting.set(p.id, run[0]!.id);
    }
    return [p];
  }

  /** The Ship approval the waiting run asks for in the app. */
  function approvals(): Approval[] {
    return [...waiting].map(([id, runId]) => ({
      id: `approval-${runId}`,
      kind: 'ship',
      runId,
      project: id,
      runNumber: Number(runId.split('-').pop()),
      step: 'Ship',
      job: null,
      assetName: null,
      service: null,
      detail: null,
      items: [],
      requestedAtMs: clock.nowMs() - 40_000,
    }));
  }

  /** Approving finishes the run and Ship writes its files; denying stops it. */
  function decide(approval: Approval, approve: boolean) {
    if (!approval.runId || waiting.get(approval.project) !== approval.runId) return;
    waiting.delete(approval.project);
    if (approve)
      for (const f of plan(approval.project).files) shipped.add(`${approval.project}:${f.path}`);
    store.runs = store.runs.map(r =>
      r.id !== approval.runId
        ? r
        : {
            ...r,
            state: approve ? 'done' : 'cancelled',
            finishedAtMs: clock.nowMs(),
            steps: r.steps.map(s =>
              s.id === 'ship' ? { ...s, state: approve ? 'done' : 'skipped' } : s
            ),
            message: approve ? null : 'Shipping was denied in Maquette.',
          }
    );
  }

  function plan(id: string): ShipPlan {
    const p = project(id);
    const game = settings(id);
    const files: ShipFile[] = [];
    for (const asset of p.assets ?? []) {
      if (!asset.versions.length) continue;
      const path = fileOf(game, asset);
      const check = checkAsset(game, asset);
      const stats = asset.versions[asset.versions.length - 1]?.stats;
      const bytes = stats?.bytes ?? null;
      const imported = game.models[asset.id];
      const moved = imported !== undefined && imported.file !== path;
      if (moved) files.push(removedFile(imported.file, bytes, stats));
      // A moved file is new at its new path, as the core plans it.
      const before = moved ? undefined : imported;
      const fixed = check ? autoFixes(check).length > 0 : false;
      const done = shipped.has(`${id}:${path}`);
      const change = done ? 'unchanged' : !before ? 'new' : fixed ? 'replaced' : 'unchanged';
      files.push({
        path,
        change,
        bytes: change === 'replaced' && bytes ? Math.round(bytes * 0.6) : bytes,
        beforeBytes: before ? bytes : null,
        triangles: change === 'replaced' && check ? check.maxTriangles : (stats?.triangles ?? null),
        beforeTriangles: before ? (stats?.triangles ?? null) : null,
        asset: asset.id,
        blocked: false,
        // Nothing shipped over it yet, so the game still has the imported file.
        imported: change === 'replaced' && Boolean(before?.importedVersion),
      });
    }
    const owned = new Set(Object.values(game.models).map(m => m.file));
    const handMade = repoModels(game.format)
      .filter(m => !owned.has(m.file) && !files.some(f => f.path === m.file))
      .map(m => m.file);
    if (game.format === 'glb') handMade.push('characters/heroine_eyes.png');
    return {
      repo: p.siteRepo,
      modelsDir: game.modelsDir,
      branch: 'main',
      exported: true,
      files: files.sort((a, b) => a.path.localeCompare(b.path)),
      handMade: handMade.map(path => ({
        path,
        change: 'hand-made',
        bytes: null,
        beforeBytes: null,
        triangles: null,
        beforeTriangles: null,
        asset: null,
        blocked: false,
        imported: false,
      })),
      recordAfter: new Set([...owned, ...files.map(f => f.path)]).size,
      checks: [
        {
          ok: files.every(
            f =>
              f.change !== 'new' ||
              !budgetFor(game, f.path) ||
              (f.triangles ?? 0) <= budgetFor(game, f.path)!.triangles
          ),
          text: 'Every model fits its budget',
        },
        { ok: true, text: 'No KTX2 or meshopt in any file' },
      ],
      // The game repo has work of its own in progress until a Ship lands.
      uncommitted: shipped.size ? 0 : 3,
    };
  }

  const handlers: Record<string, (args: Args) => unknown> = {
    game_inspect_repo: ({ path }) => inspectGameRepo(String(path)),
    game_scan: ({ project: id }): GameScan => {
      const p = project(String(id));
      const game = settings(p.id);
      return {
        repo: p.siteRepo ?? '',
        modelsDir: game.modelsDir,
        models: repoModels(game.format).map(m => {
          const asset = Object.entries(game.models).find(([, g]) => g.file === m.file)?.[0];
          return scanned(m, game, asset ?? null);
        }),
        millis: 800,
      };
    },
    game_import: ({ project: id, files }): ImportResult => {
      const p = project(String(id));
      const game = structuredClone(settings(p.id));
      const picked = new Set((files as string[]) ?? []);
      const known = new Set(Object.values(game.models).map(m => m.file));
      const models = repoModels(game.format).filter(m => picked.has(m.file) && !known.has(m.file));
      const added = models.map(m => {
        game.models[assetIdOf(m.file)] = { file: m.file, importedVersion: 1 };
        return importedAsset(m, (scanned(m, game, null).budget?.problems.length ?? 0) > 0);
      });
      const next = saveProject({ ...p, game, assets: [...(p.assets ?? []), ...added] });
      return {
        project: previews.withPreviews(
          next,
          added.map(a => a.id)
        ).project,
        imported: added.map(a => a.id),
        skipped: [...picked]
          .filter(f => known.has(f))
          .map(file => ({ file, reason: 'already imported' })),
      };
    },
    game_save_settings: ({ project: id, change }) => {
      const p = project(String(id));
      const c = change as GameSettingsChange;
      const cap = c.export?.textureCap;
      if (cap !== undefined && ![256, 512, 1024, 2048, 4096].includes(cap)) {
        fail(
          'invalid',
          `The texture size cap must be one of 256, 512, 1024, 2048 or 4096 px, not ${cap}.`
        );
      }
      const current = settings(p.id);
      const game: GameSettings = {
        ...current,
        format: c.format ?? current.format,
        modelsDir: c.modelsDir ?? current.modelsDir,
        export: c.export ?? current.export,
        bake: c.bake ?? current.bake,
        budgets: c.budgets ?? current.budgets,
        folders: withFolders(current.folders ?? {}, c.assetFolders ?? {}),
      };
      return saveProject({ ...p, game });
    },
    game_add_budget_fixups: ({ project: id }) => {
      const p = project(String(id));
      const game = settings(p.id);
      // A rigged model only over its triangles gets nothing: it is reduced by hand.
      return (p.assets ?? [])
        .filter(a => isOver(game, a) && autoFixes(checkAsset(game, a)!).length > 0)
        .map(a => a.id);
    },
    game_ship_plan: ({ project: id }) => plan(String(id)),
    ...previews.handlers,
  };

  return {
    handlers,
    /** Adds the `?game=` project and opens it. */
    seedInto: (
      projects: ProjectManifest[],
      config: WorkspaceConfig
    ): [ProjectManifest[], WorkspaceConfig] => {
      const seeded = seed();
      if (!seeded[0]) return [projects, config];
      return [[...projects, ...seeded], { ...config, currentProject: seeded[0].id }];
    },
    approvals,
    decide,
    /** What a game project's Pipeline view has in place of a website's. */
    override: (p: ProjectManifest): Partial<PipelineView> => {
      const game = gameOf(p);
      if (!game) return {};
      const def = gamePipeline(game);
      return {
        def,
        models: [],
        artifacts: [],
        starters: GAME_STARTERS.map(path => ({ path, state: 'current', backup: null })),
        bake: game.bake
          ? { settings: BAKE_DEFAULT, canDenoise: true, script: 'current', skipped: [] }
          : null,
      };
    },
    waitingRun: (id: string) => waiting.get(id) ?? null,
  };
}
