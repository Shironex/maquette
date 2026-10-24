/**
 * An in-memory stand-in for the Rust commands, keyed by the same snake_case
 * names and camelCase arguments the generated bindings send.
 *
 * Scenarios (`?scenario=` in the URL):
 * - `first-run` (default): no workspace yet, toktx missing.
 * - `configured`: a workspace with Moon Island is open.
 * - `no-blender`: like first-run, but Blender is missing.
 */
import type {
  Bootstrap,
  ErrorPayload,
  ImportScan,
  JobSpec,
  NewProject,
  ProjectManifest,
  RepoInspection,
  RootInspection,
  ToolId,
  ToolReport,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import { DEFAULT_ROOT, HOME, defaultConfig, moonIsland, toolReports } from './fixtures';

type Args = Record<string, unknown>;

export interface MockOptions {
  scenario: string;
  /** Milliseconds a toolchain check takes, so the checking state is visible. */
  probeDelay: number;
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

export function createBackend({ scenario, probeDelay }: MockOptions) {
  const configured = scenario === 'configured';
  let config: WorkspaceConfig = defaultConfig();
  let root: string | null = configured ? expand(DEFAULT_ROOT) : null;
  let projects: ProjectManifest[] = configured ? [moonIsland()] : [];
  let cacheBytes = configured ? 2.3e9 : 0;
  const overrides = new Map<ToolId, string>();
  if (configured) config = { ...config, currentProject: 'moon-island' };

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
    pipeline_submit: (args: Args) => {
      void (args.spec as JobSpec);
      fail('unavailable', 'The pipeline runner arrives in a later version.');
    },
  };

  return async function handle(cmd: string, args: Args = {}): Promise<unknown> {
    const handler = handlers[cmd];
    if (!handler) fail('unavailable', `Mock mode has no handler for ${cmd}.`);
    return handler(args);
  };
}
