/**
 * The scene layout in mock mode: Moon Island's models stand on the ground
 * around the sakura with five camera stops, each project keeps its own
 * layout in memory, and the first view counts what the first stop sees.
 */
import type {
  FirstView,
  LayoutModel,
  LayoutView,
  ModelPrep,
  Placement,
  ProjectManifest,
  SceneLayout,
  StateTrigger,
  Timeline,
} from '../client';
import { frameOf, framed } from '@/lib/real-size';

type Vec3 = [number, number, number];

/** The bundled model that stands in for each Moon Island model file. */
export const LAYOUT_FILES: Record<string, string> = {
  'models/island_v7.glb': 'sakura_tree_large.glb',
  'models/shop_house_v3.glb': 'flower_shrubs.glb',
  'models/character_v2.glb': 'character.glb',
  'models/vending_v2.glb': 'flower_shrubs.glb',
};

/** What the Rust side measures for those files: bounds in metres and bytes. */
const MEASURED: Record<string, { min: Vec3; max: Vec3; bytes: number }> = {
  'sakura_tree_large.glb': { min: [-5.84, 0, -3.82], max: [5.84, 7.97, 3.81], bytes: 330_444 },
  'flower_shrubs.glb': { min: [-0.79, 0, -0.66], max: [0.79, 0.67, 0.66], bytes: 169_808 },
  'character.glb': { min: [-0.68, 0, -0.42], max: [0.68, 2.0, 0.42], bytes: 1_511_800 },
};

/** The bounds mock mode knows for a project file. */
export function layoutBounds(file: string): { min: Vec3; max: Vec3 } | null {
  return MEASURED[LAYOUT_FILES[file] ?? ''] ?? null;
}

function moonLayout(): SceneLayout {
  return {
    version: 1,
    models: {
      island: { position: [0, 0, 0], rotation: [0, 0, 0], scale: 1 },
      'shop-house': { position: [-3.2, 0, 3.4], rotation: [0, 20, 0], scale: 1.2 },
      character: { position: [1.6, 0, 3.2], rotation: [0, -25, 0], scale: 1 },
      vending: { position: [3.6, 0, 1.2], rotation: [0, -60, 0], scale: 0.8 },
    },
    stops: [
      {
        id: 'porch',
        name: 'Porch',
        position: [6.5, 2.4, 10.5],
        target: [1, 1.4, 2.6],
        fov: 36,
        models: null,
      },
      {
        id: 'sakura',
        name: 'Sakura',
        position: [-9, 3.5, 12],
        target: [0, 4, 0],
        fov: 40,
        models: null,
      },
      {
        id: 'shrubs',
        name: 'Shrubs',
        position: [-2.4, 1.1, 9.8],
        target: [-2.8, 0.5, 3.4],
        fov: 30,
        models: null,
      },
      {
        id: 'canopy',
        name: 'Canopy',
        position: [3.4, 1.4, 5.6],
        target: [0, 5.6, -0.6],
        fov: 58,
        models: null,
      },
      {
        id: 'pullback',
        name: 'Pullback',
        position: [14, 7.5, 19],
        target: [0.5, 3, 1],
        fov: 35,
        models: null,
      },
    ],
    desk: null,
    states: {},
    clips: {},
    timeline: { explode: {} },
  };
}

const sub = (a: Vec3, b: Vec3): Vec3 => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const dot = (a: Vec3, b: Vec3) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: Vec3, b: Vec3): Vec3 => [
  a[1] * b[2] - a[2] * b[1],
  a[2] * b[0] - a[0] * b[2],
  a[0] * b[1] - a[1] * b[0],
];
const unit = (v: Vec3): Vec3 => {
  const l = Math.hypot(...v) || 1;
  return [v[0] / l, v[1] / l, v[2] / l];
};
const n = (v: number | null) => v ?? 0;
const v3 = (v: readonly (number | null)[]): Vec3 => [n(v[0]!), n(v[1]!), n(v[2]!)];

/** True when the model's scaled bounds centre falls in the stop's 16:9 frame. */
function seen(stop: SceneLayout['stops'][number], center: Vec3, radius: number): boolean {
  const eye = v3(stop.position);
  const forward = unit(sub(v3(stop.target), eye));
  const right = unit(cross(forward, [0, 1, 0]));
  const up = cross(right, forward);
  const d = sub(center, eye);
  const depth = dot(d, forward);
  if (depth + radius <= 0) return false;
  const tanV = Math.tan(((stop.fov ?? 35) * Math.PI) / 360);
  const reach = Math.max(depth, 0.01);
  return (
    Math.abs(dot(d, right)) - radius <= reach * tanV * (16 / 9) &&
    Math.abs(dot(d, up)) - radius <= reach * tanV
  );
}

export function createFakeLayout(deps: {
  project: (id: string) => ProjectManifest;
  projectDir: (id: string) => string;
  firstViewMb: () => number;
  /** Bounds of a project file, when mock mode knows them. */
  bounds: (file: string) => { min: Vec3; max: Vec3 } | null;
  /** The asset's real size and fix-ups, which move and scale it. */
  prep: (project: string, asset: string) => ModelPrep | null;
}) {
  const layouts = new Map<string, SceneLayout>([['moon-island', moonLayout()]]);

  function models(id: string): LayoutModel[] {
    const p = deps.project(id);
    const list = (p.assets ?? []).filter(a => a.versions.length > 0);
    return list.map((a, i) => {
      const latest = a.versions[a.versions.length - 1]!;
      const stand = deps.bounds(latest.file);
      const prep = frameOf(deps.prep(id, a.id), stand?.min ?? null, stand?.max ?? null);
      return {
        asset: a.id,
        name: a.name,
        kind: a.kind,
        version: latest.version,
        file: `${deps.projectDir(id)}/${latest.file}`,
        path: latest.file,
        min: stand ? framed(prep, stand.min) : null,
        max: stand ? framed(prep, stand.max) : null,
        prep,
        fallback: [i * 1.5, 0, 0],
        bytes: MEASURED[LAYOUT_FILES[latest.file] ?? '']?.bytes ?? latest.stats?.bytes ?? 0,
        measured: 'raw',
      };
    });
  }

  function placement(layout: SceneLayout, model: LayoutModel): Placement {
    return (
      layout.models[model.asset] ?? { position: model.fallback, rotation: [0, 0, 0], scale: 1 }
    );
  }

  function firstView(id: string): FirstView {
    const layout = layouts.get(id) ?? moonLayout();
    const all = models(id);
    const stop = layout.stops[0];
    const target = deps.firstViewMb();
    if (!stop) return { stop: null, custom: false, models: [], bytes: 0, targetMb: target };
    const picked = all.filter(m => {
      if (stop.models) return stop.models.includes(m.asset);
      const p = placement(layout, m);
      const s = n(p.scale);
      const min = v3(m.min ?? [0, 0, 0]);
      const max = v3(m.max ?? [0, 0, 0]);
      const center: Vec3 = [0, 1, 2].map(
        i => ((min[i]! + max[i]!) / 2) * s + n(p.position[i]!)
      ) as Vec3;
      const radius = (Math.hypot(...sub(max, min)) / 2) * s;
      return seen(stop, center, radius);
    });
    return {
      stop: stop.name,
      custom: stop.models !== null,
      models: picked.map(m => ({
        asset: m.asset,
        name: m.name,
        bytes: m.bytes,
        measured: m.measured,
      })),
      bytes: picked.reduce((sum, m) => sum + n(m.bytes), 0),
      targetMb: target,
    };
  }

  function view(id: string): LayoutView {
    return {
      project: id,
      saved: layouts.has(id),
      layout: layouts.get(id) ?? {
        version: 1,
        models: {},
        stops: [],
        desk: null,
        states: {},
        clips: {},
        timeline: { explode: {} },
      },
      models: models(id),
      firstView: firstView(id),
    };
  }

  function save(id: string, layout: SceneLayout): LayoutView {
    const known = new Set(models(id).map(m => m.asset));
    for (const asset of Object.keys(layout.models)) {
      if (!known.has(asset)) {
        throw { code: 'invalid', message: `There is no model for the asset ${asset}.` };
      }
    }
    if (layout.stops.some(s => !s.name.trim())) {
      throw { code: 'invalid', message: 'Give every camera stop a name.' };
    }
    // As the app does, the editor's save keeps the Motion and Timeline views' parts.
    const kept = layouts.get(id);
    layouts.set(id, {
      ...layout,
      states: kept?.states ?? {},
      clips: kept?.clips ?? {},
      timeline: kept?.timeline ?? { explode: {} },
    });
    return view(id);
  }

  /** The Motion view's part of the layout. */
  function motion(id: string) {
    const layout = layouts.get(id);
    const clipsOff: Record<string, string[]> = {};
    for (const [asset, clips] of Object.entries(layout?.clips ?? {})) {
      const off = Object.entries(clips ?? {}).filter(([, c]) => c && !c.ship);
      if (off.length > 0) clipsOff[asset] = off.map(([name]) => name).sort();
    }
    return { states: structuredClone(layout?.states ?? {}), clipsOff };
  }

  function setMotion(
    id: string,
    states: SceneLayout['states'],
    clipsOff: Record<string, string[]>
  ): void {
    const known = new Set(models(id).map(m => m.asset));
    for (const asset of [...Object.keys(states), ...Object.keys(clipsOff)]) {
      if (!known.has(asset)) {
        throw { code: 'invalid', message: `There is no model for the asset ${asset}.` };
      }
    }
    const current = layouts.get(id) ?? view(id).layout;
    const clips = Object.fromEntries(
      Object.entries(clipsOff)
        .filter(([, names]) => names.length > 0)
        .map(([asset, names]) => [asset, Object.fromEntries(names.map(c => [c, { ship: false }]))])
    );
    layouts.set(id, { ...current, states: structuredClone(states), clips });
  }

  /**
   * The Timeline view's save: each asset's stop and scroll triggers and the
   * whole timeline, as the app writes them.
   */
  function setTimeline(
    id: string,
    triggers: Record<string, StateTrigger[]>,
    timeline: Timeline
  ): void {
    const current = layouts.get(id) ?? view(id).layout;
    const states = structuredClone(current.states);
    for (const [asset, list] of Object.entries(triggers)) {
      const object = states[asset];
      if (!object) throw { code: 'invalid', message: `${asset} has no states yet.` };
      const kept = object.triggers.filter(t => t.kind !== 'stop' && t.kind !== 'scroll');
      states[asset] = { ...object, triggers: [...kept, ...structuredClone(list ?? [])] };
    }
    layouts.set(id, { ...current, states, timeline: structuredClone(timeline) });
  }

  /** `?timeline=empty`: the project has no camera stops. */
  function clearStops(id: string): void {
    const current = layouts.get(id) ?? view(id).layout;
    layouts.set(id, { ...current, stops: [] });
  }

  return { view, save, firstView, motion, setMotion, setTimeline, clearStops };
}
