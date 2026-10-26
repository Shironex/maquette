/**
 * An in-memory stand-in for the Rust commands, keyed by the same snake_case
 * names and camelCase arguments the generated bindings send. The scenarios
 * and URL options are listed on `BackendOptions` (backend-options.ts).
 */
import type {
  Bootstrap,
  CompressPreset,
  InboxItem,
  PipelineView,
  ProjectManifest,
  ProjectSummary,
  StarterStatus,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import { DEFAULT_ROOT, HOME, defaultConfig, mockProjects, providerCatalog } from './fixtures';
import { inboxItems } from './inbox-fixtures';
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
import { createFakeLayout, layoutBounds } from './layout';
import { createFakeBake, createPrepareHandlers } from './prepare';
import { createMaterialHandlers } from './materials';
import { createFakeProviders } from './providers';
import { createCreditHandlers } from './create-credit-handlers';
import { createDownloadsHandlers } from './create-downloads-handlers';
import { linkForRedo } from './redo-fixtures';
import { createFakeRunner, type RunStore } from './runner';
import { clock } from '@/lib/clock';
import { createAssetHandlers } from './create-asset-handlers';
import { createBuiltHandlers } from './create-built-handlers';
import { createInboxHandlers } from './create-inbox-handlers';
import { createMotionHandlers, motionConfig, seedMotionProjects } from './create-motion-handlers';
import { createPageHandlers } from './create-page-handlers';
import { createPipelineHandlers } from './create-pipeline-handlers';
import { createProjectHandlers } from './create-project-handlers';
import { createToolchainHandlers } from './create-toolchain-handlers';
import { display, expand, fail, type Args, type BackendOptions } from './backend-options';
import { inspect, validate } from './workspace-checks';

export type MockOptions = BackendOptions;

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
  inbox: inboxState = null,
  arrival = null,
  built: builtState = null,
  motion = null,
  page: pageState = null,
}: BackendOptions) {
  const configured = ['configured', 'running', 'idle'].includes(scenario);
  let config: WorkspaceConfig = defaultConfig();
  let root: string | null = configured ? expand(DEFAULT_ROOT) : null;
  let projects: ProjectManifest[] = configured
    ? seedMotionProjects(mockProjects(projectsState === 'many'), motion)
    : [];
  let cacheBytes = configured ? 2.3e9 : 0;
  if (configured) config = motionConfig({ ...config, currentProject: 'moon-island' }, motion);
  const agentsOn = agentsState === 'active' || agentsState === 'batch';
  if (configured && agentsOn) {
    config = { ...config, agents: { ...config.agents, mcpEnabled: true } };
  }

  const now = clock.nowMs();
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
  const listed = configured ? inboxItems(`${expand(DEFAULT_ROOT)}/inbox`, now) : [];
  let inbox: InboxItem[] = inboxState === 'redo' ? linkForRedo(listed) : listed;

  const prepare = createPrepareHandlers({
    project: id => project(id),
    config: () => config,
    inbox: () => inbox,
    bounds: layoutBounds,
    probeDelay,
  });
  const bake = createFakeBake(prepare.entries, id => startersOf(id));
  const layout = createFakeLayout({
    project: id => project(id),
    projectDir: id => `${root ?? expand(DEFAULT_ROOT)}/projects/${id}`,
    firstViewMb: () => config.budgets.firstViewMb ?? 3,
    bounds: prepare.bounds,
    prep: (id, asset) => prepare.entries(id).models[asset] ?? null,
  });

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
    return {
      ...base,
      def,
      active: activeRun(p.id),
      starters: startersOf(p.id),
      firstView: layout.firstView(p.id),
      bake: bake.view(p.id),
    };
  }

  /** Each project's starter scripts; the fixture Moon Island has old ones. */
  const starters = new Map<string, StarterStatus[]>();
  function startersOf(id: string): StarterStatus[] {
    // The fixture Moon Island is older than its starters, unless `?motion=`
    // shows it with clips kept off the site; a new project is not.
    const old = id === 'moon-island' && !motion && (project(id).assets ?? []).length > 0;
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
    inbox: inboxState,
    keys: inboxState === 'redo' ? ['tripo', 'meshy'] : undefined,
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

  const built = createBuiltHandlers({
    project,
    saveProject,
    emit: (id, manifest) => emit?.({ name: 'built', project: id, manifest }),
    seed: configured ? builtState : null,
    buildMs: pace === 0 ? 0 : 1400,
  });

  const page = createPageHandlers({
    project,
    saveProject,
    projectDir: id => `${root ?? expand(DEFAULT_ROOT)}/projects/${id}`,
    emit: payload => emit?.({ name: 'page', payload }),
    state: configured ? pageState : null,
    startMs: pace === 0 ? 0 : 900,
  });

  const handlers: Record<string, (args: Args) => unknown> = {
    ...lab.handlers,
    ...built.handlers,
    ...agents.handlers,
    ...providers.handlers,
    ...createCreditHandlers({
      config: () => config,
      jobs: providers.jobs,
      hasKey: providers.hasKey,
      history: configured,
    }),
    ...createDownloadsHandlers({
      jobs: providers.jobs,
      handBack: args => providers.handlers.provider_hand_back?.(args),
      emit: payload => emit?.({ name: 'downloads', payload }),
      arrival,
      ...(pace === 0 ? { delayMs: 0 } : {}),
    }),
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
    ...createToolchainHandlers(scenario, probeDelay),
    ...createProjectHandlers({
      snapshot,
      project,
      summary,
      projects: { get: () => projects, set: next => (projects = next) },
      config: { get: () => config, set: next => (config = next) },
    }),
    ...createAssetHandlers({ project, saveProject, root: () => root, files }),
    ...createPipelineHandlers({
      project,
      config: () => config,
      root: () => root,
      layout,
      runner,
      store,
      presets,
      starters,
      view,
      startersOf,
      activeRun,
      bake,
    }),
    ...createMaterialHandlers(project),
    ...createInboxHandlers({
      project,
      saveProject,
      root: () => root,
      inbox: { get: () => inbox, set: next => (inbox = next) },
      probeDelay,
      emit,
      files,
      prepare,
    }),
    ...prepare.handlers,
    ...createMotionHandlers({
      project,
      layout,
      config: () => config,
      clipsSupported: id =>
        startersOf(id).some(s => s.path === 'tools/build_scene.py' && s.state === 'current'),
      motion,
    }),
    ...page.handlers,
  };
  page.wrapInspect(handlers);

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
