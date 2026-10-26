/**
 * The Motion view in mock mode: every model's rig, clips and parts from the
 * fixtures the Rust reader reported for the bundled files, weighed inside
 * the scene budget, with the states and clip choices the fake layout keeps.
 *
 * `?motion=` in the URL:
 * - `demo`: Moon Island gains a clay figure with three clips, a switch
 *   cabinet with three states and a turntable with one clip.
 * - `rest`: like demo, but the clay figure is fresh from rigging, no clips.
 * - `over`: like demo, with a scene budget the models and clips go over.
 * - `still`: opens Rooftop cinema, where nothing moves yet; it gains the
 *   switch cabinet without states, the prop the empty view suggests.
 */
import type {
  Asset,
  MotionModel,
  MotionView,
  ObjectStates,
  ProjectManifest,
  WorkspaceConfig,
} from '../client';
import { clock } from '@/lib/clock';
import type { Args } from './backend-options';
import type { createFakeLayout } from './layout';
import { LAYOUT_FILES } from './layout';
import { MOTION_FIXTURES } from './motion-fixtures';

type FakeLayout = ReturnType<typeof createFakeLayout>;

const DEMO_MODELS = {
  'models/clay_figure_v2.glb': 'clay_figure.glb',
  'models/switch_cabinet_v1.glb': 'switch_cabinet.glb',
  'models/turntable_v1.glb': 'turntable.glb',
};

/** Project files to the bundled model each one serves, for `?motion=`. */
export function motionFiles(motion: string | null): Record<string, string> {
  if (!motion) return {};
  if (motion === 'still') return { 'models/switch_cabinet_v1.glb': 'switch_cabinet.glb' };
  return motion === 'rest'
    ? { ...DEMO_MODELS, 'models/clay_figure_v2.glb': 'clay_figure_rest.glb' }
    : DEMO_MODELS;
}

function demoAsset(id: string, name: string, kind: Asset['kind'], file: string, kb: number) {
  const at = new Date(clock.nowMs() - 3 * 3_600_000).toISOString();
  const version = Number(/_v(\d+)\.glb$/.exec(file)?.[1] ?? 1);
  const asset: Asset = {
    id,
    name,
    kind,
    stage: 'blender',
    status: 'review',
    versions: Array.from({ length: version }, (_, i) => ({
      version: i + 1,
      file: file.replace(/_v\d+\.glb$/, `_v${i + 1}.glb`),
      createdAt: at,
      stats: {
        triangles: 1_200,
        bytes: kb * 1000,
        textures: 0,
        maxTexture: 0,
        joints: 0,
        animations: 0,
      },
    })),
    thumbnail: null,
    createdAt: at,
    updatedAt: at,
  };
  return asset;
}

/** Moon Island with the demo models, when `?motion=` asks for them. */
export function seedMotionProjects(
  projects: ProjectManifest[],
  motion: string | null
): ProjectManifest[] {
  if (!motion) return projects;
  const cabinet = () =>
    demoAsset('switch-cabinet', 'Switch cabinet', 'prop', 'models/switch_cabinet_v1.glb', 10);
  if (motion === 'still') {
    return projects.map(p =>
      p.id === 'rooftop-cinema' ? { ...p, assets: [cabinet(), ...(p.assets ?? [])] } : p
    );
  }
  const extra = [
    demoAsset('clay-figure', 'Clay figure', 'character', 'models/clay_figure_v2.glb', 69),
    cabinet(),
    demoAsset('turntable', 'Turntable', 'prop', 'models/turntable_v1.glb', 8),
  ];
  return projects.map(p =>
    p.id === 'moon-island' ? { ...p, assets: [...extra, ...(p.assets ?? [])] } : p
  );
}

/** `over` tightens the scene budget; `still` opens Rooftop cinema. */
export function motionConfig(config: WorkspaceConfig, motion: string | null): WorkspaceConfig {
  if (motion === 'over') return { ...config, budgets: { ...config.budgets, sceneMb: 2.25 } };
  if (motion === 'still') return { ...config, currentProject: 'rooftop-cinema' };
  return config;
}

/** The switch cabinet's states: closed, its door open, then its breakers on. */
export function cabinetStates(): ObjectStates {
  const breakers = Object.fromEntries(
    [1, 2, 3, 4, 5, 6].map(i => [
      `Breaker ${i}`,
      {
        position: [0, 0.02, 0] as [number, number, number],
        rotation: [-30, 0, 0] as [number, number, number],
      },
    ])
  );
  const door = {
    position: [0, 0, 0] as [number, number, number],
    rotation: [0, -105, 0] as [number, number, number],
  };
  return {
    states: [
      { id: 'closed', name: 'Closed, off', parts: {} },
      { id: 'door-open', name: 'Door open', parts: { Door: door } },
      { id: 'breakers-on', name: 'Breakers on', parts: { Door: door, ...breakers } },
    ],
    transitions: [
      { from: 'closed', to: 'door-open', duration: 0.6, delay: 0, easing: 'ease-out', stagger: 0 },
      {
        from: 'door-open',
        to: 'breakers-on',
        duration: 0.4,
        delay: 0.1,
        easing: 'ease-in-out',
        stagger: 0.05,
      },
    ],
    triggers: [
      { kind: 'stop', to: 'door-open', stop: 'porch', part: null, start: null, end: null },
      { kind: 'scroll', to: 'breakers-on', stop: null, part: null, start: 40, end: 55 },
      { kind: 'click', to: 'breakers-on', stop: null, part: 'Breaker 1', start: null, end: null },
    ],
  };
}

export function createMotionHandlers(deps: {
  project: (id: string) => ProjectManifest;
  layout: FakeLayout;
  config: () => WorkspaceConfig;
  /** The project's Build scene can leave clips out (its starters are current). */
  clipsSupported: (id: string) => boolean;
  motion: string | null;
}) {
  const files = { ...LAYOUT_FILES, ...motionFiles(deps.motion) };
  if (deps.motion && deps.motion !== 'still') {
    deps.layout.setMotion('moon-island', { 'switch-cabinet': cabinetStates() }, {});
  }

  function view(id: string): MotionView {
    const layout = deps.layout.view(id);
    const assets = deps.project(id).assets ?? [];
    const models: MotionModel[] = layout.models.map(m => {
      const fixture = MOTION_FIXTURES[files[m.path] ?? ''];
      const clips = fixture?.clips ?? [];
      return {
        asset: m.asset,
        name: m.name,
        kind: m.kind,
        version: m.version,
        file: m.file,
        path: m.path,
        thumbnail: assets.find(a => a.id === m.asset)?.thumbnail ?? null,
        addedAt: new Date(clock.nowMs() - 2 * 3_600_000).toISOString(),
        bytes: m.bytes,
        measured: m.measured,
        rig: fixture?.rig ?? null,
        clips,
        motionBytes: clips.reduce((sum, c) => sum + (c.bytes ?? 0), 0),
        parts: fixture?.parts ?? [m.name],
        error: null,
      };
    });
    const firstView = layout.firstView;
    const seen = new Set(firstView.models.map(f => f.asset));
    const motion = deps.layout.motion(id);
    return {
      project: id,
      models,
      states: motion.states,
      clipsOff: motion.clipsOff,
      stops: layout.layout.stops,
      budget: {
        sceneBytes: models.reduce((sum, m) => sum + (m.bytes ?? 0), 0),
        sceneTargetMb: deps.config().budgets.sceneMb,
        motionBytes: models.reduce((sum, m) => sum + (m.motionBytes ?? 0), 0),
        firstView,
        firstViewMotion: models
          .filter(m => seen.has(m.asset))
          .reduce((sum, m) => sum + (m.motionBytes ?? 0), 0),
      },
      clipsOffSupported: deps.clipsSupported(id),
    };
  }

  return {
    motion_get: ({ project }: Args) => view(String(project)),
    motion_save: ({ project, states, clipsOff }: Args) => {
      const id = String(project);
      deps.layout.setMotion(
        id,
        states as Record<string, ObjectStates>,
        clipsOff as Record<string, string[]>
      );
      return view(id);
    },
  };
}
