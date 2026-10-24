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
 * `?pace=` sets the milliseconds between streamed log lines (default 260).
 */
import type {
  AcceptRequest,
  AssetKind,
  Bootstrap,
  CompressPreset,
  ErrorPayload,
  ImportScan,
  InboxItem,
  NewAsset,
  NewProject,
  PipelineEvent,
  PipelineView,
  Stage,
  ProjectManifest,
  RepoInspection,
  RootInspection,
  ToolId,
  ToolReport,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import { DEFAULT_ROOT, HOME, defaultConfig, moonIsland, toolReports } from './fixtures';
import { inboxItems } from './inbox-fixtures';
import { dioramaPipeline, pipelineView, run24Log, runHistory } from './pipeline-fixtures';
import { createFakeRunner, type RunStore } from './runner';

type Args = Record<string, unknown>;

export interface MockOptions {
  scenario: string;
  /** Milliseconds a toolchain check takes, so the checking state is visible. */
  probeDelay: number;
  /** Milliseconds between streamed log lines. */
  pace?: number;
  /** Delivers runner and inbox events to the UI. */
  emit?: (event: { name: 'pipeline'; payload: PipelineEvent } | { name: 'inbox' }) => void;
  /** Serves files the backend "writes", since mock mode has no disk. */
  files?: {
    /** Serves whatever `from` serves at `path` too. */
    alias: (path: string, from: string) => void;
    /** Serves `bytes` (a PNG) at `path`. */
    store: (path: string, bytes: Uint8Array) => void;
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

export function createBackend({ scenario, probeDelay, pace = 260, emit, files }: MockOptions) {
  const configured = ['configured', 'running', 'idle'].includes(scenario);
  let config: WorkspaceConfig = defaultConfig();
  let root: string | null = configured ? expand(DEFAULT_ROOT) : null;
  let projects: ProjectManifest[] = configured ? [moonIsland()] : [];
  let cacheBytes = configured ? 2.3e9 : 0;
  const overrides = new Map<ToolId, string>();
  if (configured) config = { ...config, currentProject: 'moon-island' };

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
    const def = { ...dioramaPipeline(), presets: presets.get(id) ?? dioramaPipeline().presets };
    const runs = p.id === 'moon-island' ? store.runs : [];
    const latest = runs[0];
    const base = pipelineView(
      p.id,
      '~/Maquette/projects/' + p.id,
      runs,
      latest ? (store.logs.get(latest.number) ?? []) : []
    );
    return { ...base, def, active: runner.activeId };
  }

  function snapshot(): WorkspaceSnapshot {
    if (!root) fail('invalid', 'No workspace is open yet.');
    const project = projects.find(p => p.id === config.currentProject) ?? projects[0] ?? null;
    return { root, display: display(root), config, project, projects };
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

  const handlers: Record<string, (args: Args) => unknown> = {
    app_bootstrap: (): Bootstrap => ({
      os: 'mac',
      version: '0.1.0',
      defaultRoot: DEFAULT_ROOT,
      home: HOME,
      defaultConfig: defaultConfig(),
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
      projects = [moonIsland()];
      config = { ...defaultConfig(), currentProject: 'moon-island' };
      return snapshot();
    },
    workspace_save_config: ({ config: next }) => {
      snapshot();
      validate(next as WorkspaceConfig);
      config = next as WorkspaceConfig;
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
    asset_open_in_blender: () => fail('unavailable', 'Mock mode does not open Blender.'),
    pipeline_get: ({ project: id }) => view(String(id)),
    pipeline_run: ({ project: id, from, force }) => {
      const def = view(String(id)).def;
      return runner.submit(String(id), def, (from as string | null) ?? null, Boolean(force));
    },
    pipeline_cancel: ({ runId }) => runner.cancel(String(runId)),
    pipeline_log: ({ number }) => store.logs.get(Number(number)) ?? [],
    pipeline_save_presets: ({ project: id, presets: rows }) => {
      const next = rows as CompressPreset[];
      const locked = view(String(id)).def.presets.filter(p => p.lock);
      for (const old of locked) {
        const same = next.find(p => p.model === old.model);
        if (!same || JSON.stringify(same) !== JSON.stringify(old))
          fail('invalid', old.lock ?? 'That row is locked.');
      }
      presets.set(String(id), next);
      return view(String(id));
    },
    inbox_list: () => ({
      dir: '~/Maquette/inbox',
      path: `${expand(DEFAULT_ROOT)}/inbox`,
      watching: Boolean(root),
      items: inbox.map(i => ({
        ...i,
        kind: kinds.get(i.id) ?? i.kind,
        kindGuessed: !kinds.has(i.id),
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
      return { ...item, kind: kind as AssetKind, kindGuessed: false };
    },
    inbox_accept: ({ request }) => {
      const req = request as AcceptRequest;
      const item = inbox.find(i => i.id === req.item);
      if (!item) fail('not-found', `${req.item} in the inbox`);
      const p = project(req.project);
      const assets = [...(p.assets ?? [])];
      const assetId = slugify(item.name);
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
