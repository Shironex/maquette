/**
 * The workspace commands in mock mode: bootstrap, create, open, save the
 * config and the cache numbers. The backend owns the state; these read and
 * set it through the getters and setters it passes in.
 */
import type { Bootstrap, ProjectManifest, WorkspaceConfig, WorkspaceSnapshot } from '../client';
import { DEFAULT_ROOT, HOME, defaultConfig, mockProjects, providerCatalog } from './fixtures';
import { fail, type Args } from './backend-options';
import { inspect, validate } from './workspace-checks';

interface Cell<T> {
  get: () => T;
  set: (next: T) => void;
}

interface WorkspaceContext {
  root: Cell<string | null>;
  config: Cell<WorkspaceConfig>;
  projects: Cell<ProjectManifest[]>;
  /** `many`: eight projects instead of three, also after Open. */
  many: boolean;
  /** Bytes the cache holds at the start. */
  cacheBytes: number;
  snapshot: () => WorkspaceSnapshot;
  /** A settings save changed the config. */
  saved: (config: WorkspaceConfig) => void;
}

export function createWorkspaceHandlers(ctx: WorkspaceContext) {
  let cacheBytes = ctx.cacheBytes;
  const { root, config, projects } = ctx;
  return {
    app_bootstrap: (): Bootstrap => ({
      os: 'mac',
      version: '0.1.0',
      defaultRoot: DEFAULT_ROOT,
      home: HOME,
      defaultConfig: defaultConfig(),
      catalog: providerCatalog(),
      workspace: root.get() ? ctx.snapshot() : null,
      lastWorkspaceError: null,
    }),
    workspace_inspect: ({ root: path }: Args) => inspect(String(path)),
    workspace_create: ({ root: path, config: next }: Args) => {
      const found = inspect(String(path));
      if (found.state === 'not-empty') {
        fail(
          'not-empty',
          'This folder is not empty. Pick an empty folder or an existing Maquette workspace.'
        );
      }
      validate(next as WorkspaceConfig);
      root.set(found.path);
      config.set({ ...(next as WorkspaceConfig), version: 1 });
      return ctx.snapshot();
    },
    workspace_open: ({ root: path }: Args) => {
      const found = inspect(String(path));
      if (found.state !== 'workspace') {
        fail(
          'not-a-workspace',
          'This folder is not a Maquette workspace. It has no maquette.json.'
        );
      }
      root.set(found.path);
      projects.set(mockProjects(ctx.many));
      config.set({ ...defaultConfig(), currentProject: 'moon-island' });
      return ctx.snapshot();
    },
    workspace_save_config: ({ config: next }: Args) => {
      ctx.snapshot();
      validate(next as WorkspaceConfig);
      // The switcher owns the current project; a settings save never moves it.
      const saved = { ...(next as WorkspaceConfig), currentProject: config.get().currentProject };
      config.set(saved);
      ctx.saved(saved);
      return saved;
    },
    workspace_cache_size: () => ({ bytes: cacheBytes }),
    workspace_clear_cache: () => {
      cacheBytes = 0;
      return { bytes: 0 };
    },
  } as Record<string, (args: Args) => unknown>;
}
