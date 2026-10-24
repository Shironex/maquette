/**
 * An in-memory stand-in for the Rust commands, keyed by the same snake_case
 * names and camelCase arguments the generated bindings send.
 *
 * Scenarios (`?scenario=` in the URL):
 * - `first-run` (default): no workspace yet, toktx missing.
 * - `configured`: a workspace with Moon Island is open.
 * - `no-blender`: like first-run, but Blender is missing.
 * - `running`: configured, with run 25 live at Export GLB.
 * - `idle`: configured, the last run passed.
 *
 * Configured workspaces hold Moon Island, Rooftop cinema (a Room) and
 * Lantern keeper (a Character showcase); `?projects=many` adds five more so
 * the project switcher shows its search.
 *
 * `?pace=` sets the milliseconds between streamed log lines (default 260).
 * `?lab=running` holds two concept images of the porch direction mid job.
 * `?agents=active` turns the MCP server on, with an agent's activity, a
 * run waiting at Ship for approval and a send to Tripo waiting the same way;
 * `?agents=batch` makes that send a batch of three under one approval.
 * `?handoff=batch` adds the landing workbench concept, six object refs made
 * from it and a By hand batch of them in mixed states.
 * Configured scenarios also open with Send to 3D jobs (see `providers.ts`).
 */
import type {
  AcceptRequest,
  ConceptEvent,
  AssetKind,
  Bootstrap,
  CompressPreset,
  ErrorPayload,
  McpStatus,
  ImportScan,
  InboxItem,
  NewAsset,
  NewProject,
  PipelineEvent,
  PipelineView,
  ProviderEvent,
  Stage,
  ProjectManifest,
  ProjectSummary,
  StarterStatus,
  RepoInspection,
  RootInspection,
  ToolId,
  ToolReport,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import {
  DEFAULT_ROOT,
  HOME,
  defaultConfig,
  mockProjects,
  providerCatalog,
  toolReports,
} from './fixtures';
import { inboxItems } from './inbox-fixtures';
import { inspectView } from './inspect-fixtures';
import {
  currentStarters,
  dioramaPipeline,
  oldStarters,
  pipelineView,
  run24Log,
  runHistory,
} from './pipeline-fixtures';
import { createFakeAgents } from './agents';
import { createFakeLab, refMayReplace } from './lab';
import { createFakeProviders } from './providers';
import { createFakeRunner, type RunStore } from './runner';

type Args = Record<string, unknown>;

export interface MockOptions {
  scenario: string;
  /** Milliseconds a toolchain check takes, so the checking state is visible. */
  probeDelay: number;
  /** Milliseconds between streamed log lines. */
  pace?: number;
  /** Delivers runner, inbox and concept events to the UI. */
  emit?: (
    event:
      | { name: 'pipeline'; payload: PipelineEvent }
      | { name: 'inbox' }
      | { name: 'concept'; payload: ConceptEvent }
      | { name: 'mcp'; payload: McpStatus }
      | { name: 'provider'; payload: ProviderEvent }
  ) => void;
  /** `active`: the MCP server is on, mid agent session. */
  agents?: string | null;
  /** `running` holds two concept jobs mid way, for screenshots. */
  lab?: string | null;
  /** Milliseconds one fake concept image takes (default 4000). */
  conceptMs?: number;
  /** `many`: eight projects instead of three. */
  projects?: string | null;
  /** `batch`: the workbench objects and their By hand batch. */
  handoff?: string | null;
  /** Serves files the backend "writes", since mock mode has no disk. */
  files?: {
    /** Serves whatever `from` serves at `path` too. */
    alias: (path: string, from: string) => void;
    /** Serves `bytes` (a PNG) at `path`. */
    store: (path: string, bytes: Uint8Array) => void;
    /** Tiles the images served at `sources` into one PNG served at `path`. */
    sheet?: (path: string, sources: string[]) => Promise<void>;
  };
}

function fail(code: string, message: string): never {
  const payload: ErrorPayload = { code, message };
  throw payload;
}

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

function expand(path: string): string {
  return path.replace(/^~(?=$|\/)/, HOME);
}

function display(path: string): string {
  return path.startsWith(HOME) ? `~${path.slice(HOME.length)}` : path;
}

function slugify(name: string): string {
  return (
    name
      .toLowerCase()
      .normalize('NFKD')
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || 'project'
  );
}

export function createBackend({
  scenario,
  probeDelay,
  pace = 260,
  emit,
  files,
  lab: labState = null,
  conceptMs = 4000,
  agents: agentsState = null,
  projects: projectsState = null,
  handoff = null,
}: MockOptions) {
  const configured = ['configured', 'running', 'idle'].includes(scenario);
  let config: WorkspaceConfig = defaultConfig();
  let root: string | null = configured ? expand(DEFAULT_ROOT) : null;
  let projects: ProjectManifest[] = configured ? mockProjects(projectsState === 'many') : [];
  let cacheBytes = configured ? 2.3e9 : 0;
  const overrides = new Map<ToolId, string>();
  if (configured) config = { ...config, currentProject: 'moon-island' };
  const agentsOn = agentsState === 'active' || agentsState === 'batch';
  if (configured && agentsOn) {
    config = { ...config, agents: { ...config.agents, mcpEnabled: true } };
  }

  const now = Date.now();
  const history = configured ? runHistory(now) : [];
  const store: RunStore = {
    runs: scenario === 'idle' ? history.slice(1) : history,
    logs: new Map(configured && scenario !== 'idle' ? [[24, run24Log(now)]] : []),
  };
  const presets = new Map<string, CompressPreset[]>();
  const runner = createFakeRunner(store, {
    emit: payload => emit?.({ name: 'pipeline', payload }),
    pace,
    holdAt: scenario === 'running' ? 'export' : undefined,
    postersFail: scenario !== 'idle',
  });
  let inbox: InboxItem[] = configured ? inboxItems(`${expand(DEFAULT_ROOT)}/inbox`, now) : [];
  const kinds = new Map<string, AssetKind>();

  function project(id: string): ProjectManifest {
    const found = projects.find(p => p.id === id);
    if (!found) fail('not-found', `Could not find the project ${id}.`);
    return found;
  }

  function saveProject(next: ProjectManifest): ProjectManifest {
    projects = projects.map(p => (p.id === next.id ? next : p));
    return next;
  }

  function view(id: string): PipelineView {
    const p = project(id);
    // Like the app: the character row only once the project has a character.
    const character = (p.assets ?? []).some(a => a.kind === 'character');
    const def = {
      ...dioramaPipeline(),
      presets: presets.get(id) ?? (character ? dioramaPipeline().presets : []),
    };
    const runs = store.runs.filter(r => r.project === p.id);
    const latest = runs[0];
    const base = pipelineView(
      p.id,
      '~/Maquette/projects/' + p.id,
      runs,
      latest ? (store.logs.get(latest.number) ?? []) : []
    );
    return { ...base, def, active: activeRun(p.id), starters: startersOf(p.id) };
  }

  /** Each project's starter scripts; the fixture Moon Island has old ones. */
  const starters = new Map<string, StarterStatus[]>();
  function startersOf(id: string): StarterStatus[] {
    // The fixture Moon Island is older than its starters; a new project is not.
    const old = id === 'moon-island' && (project(id).assets ?? []).length > 0;
    if (!starters.has(id)) starters.set(id, old ? oldStarters() : currentStarters());
    return starters.get(id)!;
  }

  /** The mock runner runs one at a time; this is its run when it belongs to `id`. */
  function activeRun(id: string): string | null {
    const active = runner.activeId;
    return active?.startsWith(`${id}-run-`) ? active : null;
  }

  /** Cards an older Send picks to Refs left without a picture get the newest ref. */
  function heal(p: ProjectManifest): ProjectManifest {
    let changed = false;
    const assets = (p.assets ?? []).map(a => {
      if (a.thumbnail !== null || a.versions.length > 0) return a;
      const newest = lab.sentFor(p.id, a.name).at(-1);
      if (!newest || !refMayReplace(a.thumbnail)) return a;
      changed = true;
      return { ...a, thumbnail: newest };
    });
    return changed ? saveProject({ ...p, assets }) : p;
  }

  function snapshot(): WorkspaceSnapshot {
    if (!root) fail('invalid', 'No workspace is open yet.');
    const open = projects.find(p => p.id === config.currentProject) ?? projects[0] ?? null;
    const project = open ? heal(open) : null;
    return { root, display: display(root), config, project, projects };
  }

  function summary(p: ProjectManifest): ProjectSummary {
    const dir = `${expand(DEFAULT_ROOT)}/projects/${p.id}`;
    const pictured = (p.assets ?? []).filter(a => a.thumbnail);
    const thumb =
      pictured.find(a => a.kind === 'scene')?.thumbnail ??
      [...pictured].sort((a, b) => b.updatedAt.localeCompare(a.updatedAt))[0]?.thumbnail ??
      null;
    const sends = (providers.jobs() ?? []).filter(
      j => j.project === p.id && ['approval', 'queued', 'running', 'waiting'].includes(j.state)
    ).length;
    return {
      id: p.id,
      name: p.name,
      template: p.template,
      siteRepo: p.siteRepo,
      createdAt: p.createdAt,
      assetCount: (p.assets ?? []).length,
      dir,
      thumb,
      activity: { run: activeRun(p.id), concepts: lab.activeJobs(p.id), sends },
    };
  }

  function inspect(path: string): RootInspection {
    const full = expand(path.trim());
    const base = { path: full, display: display(full) };
    if (/existing/i.test(full)) return { ...base, state: 'workspace', entries: 4 };
    if (/Documents$|Desktop$|Downloads$/.test(full) || full === HOME) {
      return { ...base, state: 'not-empty', entries: 12 };
    }
    if (full === expand(DEFAULT_ROOT)) return { ...base, state: 'empty', entries: 0 };
    return { ...base, state: 'missing', entries: 0 };
  }

  function tools(): ToolReport[] {
    return toolReports(scenario).map(r => {
      const path = overrides.get(r.id);
      return path
        ? {
            ...r,
            status: 'found',
            source: 'override',
            path,
            version: r.version ?? '1.0.0',
            detail: r.detail === 'not found on PATH' ? null : r.detail,
            hint: null,
          }
        : r;
    });
  }

  function validate(next: WorkspaceConfig) {
    const port = next.agents.mcpPort;
    if (port < 1024 || port > 65535)
      fail('invalid', 'The MCP port must be between 1024 and 65535.');
    if (!((next.budgets.sceneMb ?? 0) > 0))
      fail('invalid', 'The scene budget must be more than 0 MB and at most 1000 MB.');
  }

  const lab = createFakeLab({
    projectDir: id => `${expand(DEFAULT_ROOT)}/projects/${id}`,
    project,
    saveProject,
    emit: payload => emit?.({ name: 'concept', payload }),
    files,
    jobMs: conceptMs,
    state: labState,
    handoff,
    fail,
    assetBusy: (project, asset) =>
      providers
        .jobs()
        .some(
          j =>
            j.project === project &&
            j.asset === asset &&
            ['approval', 'queued', 'running', 'waiting', 'inbox'].includes(j.state)
        ),
  });

  const providers = createFakeProviders({
    config: () => config,
    configured,
    agentsActive: agentsOn,
    agentsBatch: agentsState === 'batch',
    handoff,
    projectDir: id => `${expand(DEFAULT_ROOT)}/projects/${id}`,
    project,
    saveProject,
    lab,
    emit: payload => emit?.({ name: 'provider', payload }),
    fail,
  });

  const agents = createFakeAgents({
    config: () => config,
    active: agentsOn,
    batch: agentsState === 'batch',
    emit: payload => emit?.({ name: 'mcp', payload }),
    onSendDecision: providers.decide,
  });

  const handlers: Record<string, (args: Args) => unknown> = {
    ...lab.handlers,
    ...agents.handlers,
    ...providers.handlers,
    // Nothing closes in the browser, so the quit commands have nothing to do.
    quit_set_unapplied: () => undefined,
    quit_now: () => undefined,
    app_bootstrap: (): Bootstrap => ({
      os: 'mac',
      version: '0.1.0',
      defaultRoot: DEFAULT_ROOT,
      home: HOME,
      defaultConfig: defaultConfig(),
      catalog: providerCatalog(),
      workspace: root ? snapshot() : null,
      lastWorkspaceError: null,
    }),
    workspace_inspect: ({ root: path }) => inspect(String(path)),
    workspace_create: ({ root: path, config: next }) => {
      const found = inspect(String(path));
      if (found.state === 'not-empty') {
        fail(
          'not-empty',
          'This folder is not empty. Pick an empty folder or an existing Maquette workspace.'
        );
      }
      validate(next as WorkspaceConfig);
      root = found.path;
      config = { ...(next as WorkspaceConfig), version: 1 };
      return snapshot();
    },
    workspace_open: ({ root: path }) => {
      const found = inspect(String(path));
      if (found.state !== 'workspace') {
        fail(
          'not-a-workspace',
          'This folder is not a Maquette workspace. It has no maquette.json.'
        );
      }
      root = found.path;
      projects = mockProjects(projectsState === 'many');
      config = { ...defaultConfig(), currentProject: 'moon-island' };
      return snapshot();
    },
    workspace_save_config: ({ config: next }) => {
      snapshot();
      validate(next as WorkspaceConfig);
      // The switcher owns the current project; a settings save never moves it.
      config = { ...(next as WorkspaceConfig), currentProject: config.currentProject };
      lab.setConcurrency(config.lab.concurrency);
      agents.changed();
      return config;
    },
    workspace_cache_size: () => ({ bytes: cacheBytes }),
    workspace_clear_cache: () => {
      cacheBytes = 0;
      return { bytes: 0 };
    },
    toolchain_detect: async () => {
      await sleep(probeDelay);
      return tools();
    },
    toolchain_locate: async ({ tool, path }) => {
      await sleep(probeDelay / 2);
      overrides.set(tool as ToolId, String(path));
      const found = tools().find(r => r.id === tool);
      if (!found) fail('not-found', `Could not find ${String(tool)}.`);
      return found;
    },
    toolchain_reset: ({ tool }) => {
      overrides.delete(tool as ToolId);
      return tools().find(r => r.id === tool);
    },
    project_create: ({ project }) => {
      snapshot();
      const next = project as NewProject;
      const name = next.name.trim();
      if (!name) fail('invalid', 'Give the project a name.');
      let id = slugify(name);
      while (projects.some(p => p.id === id)) id = `${id}-2`;
      const manifest: ProjectManifest = {
        version: 1,
        id,
        name,
        template: next.template,
        siteRepo: next.siteRepo ? expand(next.siteRepo) : null,
        sourceFolder: next.sourceFolder ? expand(next.sourceFolder) : null,
        createdAt: new Date().toISOString(),
        assets: [],
      };
      projects = [...projects, manifest];
      config = { ...config, currentProject: id };
      return snapshot();
    },
    project_list: () => {
      snapshot();
      return projects.map(summary);
    },
    project_switch: ({ id }) => {
      snapshot();
      project(String(id));
      config = { ...config, currentProject: String(id) };
      return snapshot();
    },
    project_update: ({ id, name, siteRepo }) => {
      const current = projects.find(p => p.id === id);
      if (!current) fail('not-found', `Could not find the project ${String(id)}.`);
      const updated: ProjectManifest = {
        ...current,
        name: String(name).trim() || current.name,
        siteRepo: siteRepo ? expand(String(siteRepo)) : null,
      };
      projects = projects.map(p => (p.id === id ? updated : p));
      return updated;
    },
    project_inspect_repo: ({ path }): RepoInspection => {
      const full = expand(String(path));
      const known = /site|landing|web/i.test(full);
      return { path: full, exists: known, isGit: known, hasPublicDir: known };
    },
    project_scan_import: ({ path }): ImportScan => ({
      path: expand(String(path)),
      blendFiles: [
        { file: 'scenes/island_v3.blend', stem: 'island', version: 3 },
        { file: 'scenes/island_v2.blend', stem: 'island', version: 2 },
      ],
      toolScripts: ['bake.py', 'export.py'],
    }),
    dialog_pick_folder: ({ start }) => (typeof start === 'string' && start ? start : DEFAULT_ROOT),
    dialog_pick_file: () => '/usr/local/bin/toktx',
    dialog_pick_model: () => `${HOME}/Downloads/desk_lamp.glb`,
    asset_add: ({ project: id, asset }) => {
      const p = project(String(id));
      const next = asset as NewAsset;
      const name = next.name.trim();
      if (!name) fail('invalid', 'Give the asset a name.');
      let assetId = slugify(name);
      while ((p.assets ?? []).some(a => a.id === assetId)) assetId = `${assetId}-2`;
      const at = new Date().toISOString();
      return saveProject({
        ...p,
        assets: [
          ...(p.assets ?? []),
          {
            id: assetId,
            name,
            kind: next.kind,
            stage: next.stage,
            status: 'queued',
            versions: [],
            thumbnail: null,
            createdAt: at,
            updatedAt: at,
          },
        ],
      });
    },
    asset_move: ({ project: id, id: assetId, stage }) => {
      const p = project(String(id));
      if (!(p.assets ?? []).some(a => a.id === assetId))
        fail('not-found', `Could not find the asset ${String(assetId)}.`);
      const at = new Date().toISOString();
      return saveProject({
        ...p,
        assets: (p.assets ?? []).map(a =>
          a.id === assetId ? { ...a, stage: stage as Stage, updatedAt: at } : a
        ),
      });
    },
    asset_rename: ({ project: id, id: assetId, name }) => {
      const p = project(String(id));
      const next = String(name).trim();
      if (!next) fail('invalid', 'Give the asset a name.');
      if (next.length > 80) fail('invalid', 'Keep the asset name to 80 characters or fewer.');
      const assets = p.assets ?? [];
      if (!assets.some(a => a.id === assetId))
        fail('not-found', `Could not find the asset ${String(assetId)}.`);
      const slug = slugify(next);
      if (assets.some(a => a.id !== assetId && (a.id === slug || slugify(a.name) === slug)))
        fail('invalid', `Another asset is already called ${next}.`);
      const at = new Date().toISOString();
      return saveProject({
        ...p,
        assets: assets.map(a => (a.id === assetId ? { ...a, name: next, updatedAt: at } : a)),
      });
    },
    asset_open_in_blender: () => fail('unavailable', 'Mock mode does not open Blender.'),
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
            sceneMb: config.budgets.sceneMb ?? 3,
            trisPerModel: config.budgets.trisPerModel,
          },
          scene: { bytes: 0, triangles: 0, models: 0 },
        };
      }
      return inspectView(
        p.id,
        `${root ?? expand(DEFAULT_ROOT)}/projects/${p.id}`,
        v.def.presets,
        v.defaults,
        config.budgets.sceneMb ?? 3,
        config.budgets.trisPerModel
      );
    },
    inspect_export_glb: ({ project: id, model }) => {
      project(String(id));
      return { model: String(model), file: `web/raw/${String(model)}.glb`, runNumber: 27 };
    },
    inbox_list: () => ({
      dir: '~/Maquette/inbox',
      path: `${expand(DEFAULT_ROOT)}/inbox`,
      watching: Boolean(root),
      items: inbox.map(i => ({
        ...i,
        kind: kinds.get(i.id) ?? i.kind,
        kindGuessed: !kinds.has(i.id) && !i.kindFromSend,
      })),
    }),
    inbox_reprocess: async ({ id }) => {
      await sleep(probeDelay / 2);
      const item = inbox.find(i => i.id === id);
      if (!item) fail('not-found', `${String(id)} in the inbox`);
      return item;
    },
    inbox_set_kind: ({ id, kind }) => {
      const item = inbox.find(i => i.id === id);
      if (!item) fail('not-found', `${String(id)} in the inbox`);
      kinds.set(item.id, kind as AssetKind);
      return { ...item, kind: kind as AssetKind, kindGuessed: false, kindFromSend: false };
    },
    inbox_accept: ({ request }) => {
      const req = request as AcceptRequest;
      const item = inbox.find(i => i.id === req.item);
      if (!item) fail('not-found', `${req.item} in the inbox`);
      const p = project(req.project);
      const assets = [...(p.assets ?? [])];
      const assetId = req.asset ?? slugify(item.name);
      let target = assets.find(a => a.id === assetId);
      const at = new Date().toISOString();
      if (!target) {
        target = {
          id: assetId,
          name: item.name,
          kind: kinds.get(item.id) ?? item.kind,
          stage: 'tripo',
          status: 'queued',
          versions: [],
          thumbnail: null,
          createdAt: at,
          updatedAt: at,
        };
        assets.push(target);
      }
      const number = target.versions.length + 1;
      const version = {
        version: number,
        file: `models/${assetId.replace(/-/g, '_')}_v${number}.glb`,
        createdAt: at,
        stats: item.report
          ? {
              triangles: item.report.triangles,
              bytes: item.report.bytes,
              textures: item.report.textures.length,
              maxTexture: item.report.maxTexture,
              joints: item.report.joints,
              animations: item.report.animations.length,
            }
          : null,
      };
      // The service renders a thumbnail for the new version; mock mode reuses the inbox one.
      let thumbnail = target.thumbnail;
      if (item.thumbnail) {
        thumbnail = `thumbs/${assetId.replace(/-/g, '_')}_v${number}.png`;
        files?.alias(`${expand(DEFAULT_ROOT)}/projects/${p.id}/${thumbnail}`, item.thumbnail);
      }
      const updated = {
        ...target,
        thumbnail,
        versions: [...target.versions, version],
        stage: 'blender' as Stage,
        status: 'review' as const,
        updatedAt: at,
      };
      const next = saveProject({ ...p, assets: assets.map(a => (a.id === assetId ? updated : a)) });
      inbox = inbox.filter(i => i.id !== item.id);
      emit?.({ name: 'inbox' });
      return {
        asset: updated,
        version,
        file: `${expand(DEFAULT_ROOT)}/projects/${p.id}/${version.file}`,
        project: next,
      };
    },
    inbox_open_in_blender: () => fail('unavailable', 'Mock mode does not open Blender.'),
    thumbnail_store: ({ file, png }) => {
      const item = inbox.find(i => i.file === file);
      if (!item) fail('not-found', `${String(file)} in the inbox`);
      const path = `${expand(DEFAULT_ROOT)}/cache/thumbs/${item.id}.png`;
      files?.store(path, Uint8Array.from(png as number[]));
      inbox = inbox.map(i =>
        i.id === item.id ? { ...i, thumbnail: path, thumbnailState: 'ready' as const } : i
      );
      return path;
    },
  };

  // The `running` scenario starts run 25 as soon as the app opens.
  if (scenario === 'running') {
    setTimeout(() => runner.submit('moon-island', view('moon-island').def, null, false), 50);
  }

  return async function handle(cmd: string, args: Args = {}): Promise<unknown> {
    const handler = handlers[cmd];
    if (!handler) fail('unavailable', `Mock mode has no handler for ${cmd}.`);
    return handler(args);
  };
}
