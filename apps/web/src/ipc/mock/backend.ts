/**
 * An in-memory stand-in for the Rust commands, keyed by the same snake_case
 * names and camelCase arguments the generated bindings send. The scenarios
 * and URL options are listed on `BackendOptions` (backend-options.ts).
 */
import type {
  CompressPreset,
  InboxItem,
  PipelineView,
  ProjectManifest,
  ProjectSummary,
  StarterStatus,
  WorkspaceConfig,
  WorkspaceSnapshot,
} from '../client';
import { DEFAULT_ROOT, defaultConfig, mockProjects } from './fixtures';
import { inboxItems } from './inbox-fixtures';
import { linkGameInbox } from './game-inbox';
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
import { projectSummary } from './project-summary';
import { createFakeBake, createPrepareHandlers } from './prepare';
import { createMaterialHandlers } from './materials';
import { createFakeProviders } from './providers';
import { createCreditHandlers } from './create-credit-handlers';
import { createDownloadsHandlers } from './create-downloads-handlers';
import { linkForRedo } from './redo-fixtures';
import { createFakeRunner, type RunStore } from './runner';
import { clock } from '@/lib/clock';
import { createAssetHandlers } from './create-asset-handlers';
import { createTrashHandlers } from './create-trash-handlers';
import { createBuiltHandlers } from './create-built-handlers';
import { createInboxHandlers } from './create-inbox-handlers';
import { createMotionHandlers, motionConfig, seedMotionProjects } from './create-motion-handlers';
import { createPageHandlers } from './create-page-handlers';
import { createTimelineHandlers } from './create-timeline-handlers';
import { createPathsHandlers } from './create-paths-handlers';
import { createGameHandlers } from './create-game-handlers';
import { withManyAssets, wrapManyInspect } from './many-assets-fixtures';
import { manyPipeline, seedManyPipeline } from './pipeline-many';
import { createLibraryHandlers } from './create-library-handlers';
import { createPipelineHandlers } from './create-pipeline-handlers';
import { createShipHandlers } from './create-ship-handlers';
import { createProjectHandlers } from './create-project-handlers';
import { createToolchainHandlers } from './create-toolchain-handlers';
import { createAppHandlers } from './create-app-handlers';
import { createWorkspaceHandlers } from './create-workspace-handlers';
import { createSyncHandlers, syncProjects } from './create-sync-handlers';
import { display, expand, fail, type Args, type BackendOptions } from './backend-options';

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
  timeline = null,
  paths = null,
  game: gameState = null,
  library: libraryState = null,
  inspect: inspectState = null,
  sync = null,
  ship: shipState = null,
  updater = null,
  pipeline: pipelineState = null,
}: BackendOptions) {
  const configured = ['configured', 'running', 'idle'].includes(scenario);
  let config: WorkspaceConfig = defaultConfig();
  let root: string | null = configured ? expand(DEFAULT_ROOT) : null;
  let projects: ProjectManifest[] = configured
    ? syncProjects(seedMotionProjects(mockProjects(projectsState === 'many'), motion), sync)
    : [];
  if (configured) config = motionConfig({ ...config, currentProject: 'frostveil' }, motion);
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
  const game = createGameHandlers({
    project: id => project(id),
    saveProject: next => saveProject(next),
    store,
    files,
    state: configured ? gameState : null,
  });
  [projects, config] = game.seedInto(projects, config);
  if (configured && inspectState === 'many') {
    projects = projects.map(p => (p.id === 'frostveil' ? withManyAssets(p) : p));
  }
  if (configured) projects = projects.map(p => seedManyPipeline(p, pipelineState));
  const listed = configured ? inboxItems(`${expand(DEFAULT_ROOT)}/inbox`, now) : [];
  let inbox: InboxItem[] = linkGameInbox(
    inboxState === 'redo' ? linkForRedo(listed) : listed,
    gameState === 'crypt' ? (projects.find(p => p.id === 'crypt-game') ?? null) : null
  );

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
    const log = runs[0] ? (store.logs.get(runs[0].number) ?? []) : [];
    const base = pipelineView(p.id, '~/Maquette/projects/' + p.id, runs, log);
    const full: PipelineView = {
      ...base,
      def,
      active: activeRun(p.id) ?? game.waitingRun(p.id),
      starters: startersOf(p.id),
      firstView: layout.firstView(p.id),
      bake: bake.view(p.id),
      ...game.override(p),
    };
    return manyPipeline(full, p, pipelineState);
  }

  /** Each project's starter scripts; the fixture Frostveil has old ones. */
  const starters = new Map<string, StarterStatus[]>();
  function startersOf(id: string): StarterStatus[] {
    // The fixture Frostveil is older than its starters, unless `?motion=`
    // shows it with clips kept off the site; a new project is not.
    const old = id === 'frostveil' && !motion && (project(id).assets ?? []).length > 0;
    if (!starters.has(id)) starters.set(id, old ? oldStarters() : currentStarters());
    return starters.get(id)!;
  }

  const activeRun = (id: string) => runner.activeFor(id);

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

  const summary = (p: ProjectManifest): ProjectSummary =>
    projectSummary(p, {
      activeRun,
      concepts: id => lab.activeJobs(id),
      jobs: () => providers.jobs(),
    });

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
    games: game,
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

  const library = createLibraryHandlers({
    projects: () => projects,
    project,
    saveProject,
    root: () => root ?? expand(DEFAULT_ROOT),
    prep: (id, asset) => prepare.entries(id).models[asset] ?? null,
    savePrep: (id, asset, entry) => prepare.handlers.prepare_save?.({ project: id, asset, entry }),
    alias: (path, from) => files?.alias(path, from),
    state: configured ? libraryState : null,
  });

  const handlers: Record<string, (args: Args) => unknown> = {
    ...lab.handlers,
    ...library.handlers,
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
      project,
      emit: payload => emit?.({ name: 'downloads', payload }),
      arrival,
      ...(pace === 0 ? { delayMs: 0 } : {}),
    }),
    ...createAppHandlers({ state: updater, emit: payload => emit?.({ name: 'updater', payload }) }),
    ...createWorkspaceHandlers({
      root: { get: () => root, set: next => (root = next) },
      config: { get: () => config, set: next => (config = next) },
      projects: { get: () => projects, set: next => (projects = next) },
      many: projectsState === 'many',
      sync,
      cacheBytes: configured ? 2.3e9 : 0,
      snapshot,
      saved: next => {
        lab.setConcurrency(next.lab.concurrency);
        agents.changed();
      },
    }),
    ...createToolchainHandlers(scenario, probeDelay),
    ...createSyncHandlers(sync),
    ...createProjectHandlers({
      snapshot,
      project,
      summary,
      projects: { get: () => projects, set: next => (projects = next) },
      config: { get: () => config, set: next => (config = next) },
    }),
    ...createAssetHandlers({ project, saveProject, root: () => root, files }),
    ...createTrashHandlers({
      project,
      saveProject,
      jobs: providers.jobs,
      stopJob: id => providers.handlers.provider_cancel?.({ id }),
      activeRun,
    }),
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
    ...createShipHandlers({ project, view, state: shipState }),
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
    ...createTimelineHandlers({ layout, timeline: configured ? timeline : null }),
    ...createPathsHandlers({ layout, paths: configured ? paths : null }),
    ...page.handlers,
    ...game.handlers,
  };
  page.wrapInspect(handlers);
  if (configured && inspectState === 'many') wrapManyInspect(handlers, project);

  // The `running` scenario starts run 25 as soon as the app opens.
  if (scenario === 'running') {
    setTimeout(() => runner.submit('frostveil', view('frostveil').def, null, false), 50);
  }

  return async function handle(cmd: string, args: Args = {}): Promise<unknown> {
    const handler = handlers[cmd];
    if (!handler) fail('unavailable', `Mock mode has no handler for ${cmd}.`);
    return handler(args);
  };
}
