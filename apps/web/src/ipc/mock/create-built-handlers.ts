/**
 * Built assets in mock mode: models a Blender script in the project makes.
 * `?built=demo` puts two on Frostveil at Blender, a laptop that is current
 * and a Newton's cradle whose script changed since its last build. Rebuild
 * takes a moment, then makes the next version, as the app does headless.
 */
import type {
  Asset,
  BuildOutcome,
  BuildRecord,
  BuiltAsset,
  BuiltSource,
  NewBuilt,
  ProjectManifest,
} from '../client';
import { clock } from '@/lib/clock';
import { HOME } from './mock-paths';
import { fail, slugify, type Args } from './backend-options';

interface BuiltContext {
  project: (id: string) => ProjectManifest;
  saveProject: (next: ProjectManifest) => ProjectManifest;
  /** Tells the UI the project's built assets changed, with the project when it did too. */
  emit: (project: string, manifest?: ProjectManifest) => void;
  seed: string | null;
  /** Milliseconds a fake build takes. */
  buildMs: number;
}

/** Where the dialog's pickers point: a folder of piece scripts. */
export const PIECES = `${HOME}/Documents/room/scripts`;

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

function record(version: number | null, objects: string[], materials: string[]): BuildRecord {
  return {
    key: `mock-${version ?? 0}`,
    at: clock.now().toISOString(),
    version,
    error: null,
    objects,
    materials,
    triangles: 1_840,
    warnings: [],
  };
}

const SEED: readonly { id: string; name: string; script: string; objects: string[] }[] = [
  { id: 'laptop', name: 'Laptop', script: 'laptop.py', objects: ['laptop', 'lid', 'screen'] },
  {
    id: 'newtons-cradle',
    name: 'Newtons cradle',
    script: 'newtons_cradle.py',
    objects: ['newtons_cradle', 'cradle_frame', 'ball_1', 'ball_2', 'ball_3'],
  },
];

function seedAsset(id: string, name: string): Asset {
  const at = new Date(clock.nowMs() - 3_600_000).toISOString();
  return {
    id,
    name,
    kind: 'prop',
    stage: 'blender',
    status: 'done',
    versions: [
      {
        version: 1,
        file: `models/${id.replace(/-/g, '_')}_v1.glb`,
        createdAt: at,
        stats: {
          triangles: 1_840,
          bytes: 96_000,
          textures: 1,
          maxTexture: 64,
          joints: 0,
          animations: 0,
        },
      },
    ],
    thumbnail: null,
    createdAt: at,
    updatedAt: at,
  };
}

/** The built commands, and the two file pickers the Add built asset dialog uses. */
export function createBuiltHandlers({ project, saveProject, emit, seed, buildMs }: BuiltContext) {
  const sources = new Map<string, Map<string, BuiltSource>>();
  /** `project/asset` of the builds running now. */
  const building = new Set<string>();
  /** `project/asset` whose files changed since their last build. */
  const stale = new Set<string>();

  const of = (id: string) => {
    if (!sources.has(id)) sources.set(id, new Map());
    return sources.get(id)!;
  };

  if (seed === 'demo') {
    const moon = project('frostveil');
    const assets = SEED.map(s => seedAsset(s.id, s.name));
    saveProject({ ...moon, assets: [...(moon.assets ?? []), ...assets] });
    for (const s of SEED) {
      of('frostveil').set(s.id, {
        script: `built/${s.script}`,
        helpers: ['built/common.py'],
        root: null,
        last: record(1, s.objects, ['palette', ...(s.id === 'laptop' ? ['screen'] : [])]),
      });
    }
    stale.add('frostveil/newtons-cradle');
  }

  function list(id: string): BuiltAsset[] {
    const p = project(id);
    return [...of(id)].flatMap(([asset, source]) => {
      const found = (p.assets ?? []).find(a => a.id === asset);
      if (!found) return [];
      const key = `${id}/${asset}`;
      const state = building.has(key)
        ? 'building'
        : !source.last
          ? 'new'
          : source.last.error
            ? 'failed'
            : stale.has(key)
              ? 'stale'
              : 'current';
      return [{ asset, name: found.name, source, state, missing: [] }];
    });
  }

  function setStatus(id: string, asset: string, status: Asset['status']): ProjectManifest {
    const p = project(id);
    return saveProject({
      ...p,
      assets: (p.assets ?? []).map(a => (a.id === asset ? { ...a, status } : a)),
    });
  }

  async function rebuild(id: string, asset: string, force: boolean): Promise<BuildOutcome> {
    const source = of(id).get(asset);
    if (!source) fail('invalid', `${asset} is not a built asset.`);
    const key = `${id}/${asset}`;
    const current = source.last && !source.last.error && !stale.has(key);
    const find = (p: ProjectManifest) => (p.assets ?? []).find(a => a.id === asset)!;
    if (current && !force) {
      const p = project(id);
      return {
        result: 'up-to-date',
        asset: find(p),
        project: p,
        version: null,
        record: source.last,
        log: [],
      };
    }
    building.add(key);
    emit(id, setStatus(id, asset, 'running'));
    await new Promise(resolve => setTimeout(resolve, buildMs));
    building.delete(key);
    stale.delete(key);
    const p = project(id);
    const before = find(p);
    const number = before.versions.length + 1;
    const version = {
      version: number,
      file: `models/${asset.replace(/-/g, '_')}_v${number}.glb`,
      createdAt: clock.now().toISOString(),
      stats: {
        triangles: 1_840,
        bytes: 96_000,
        textures: 1,
        maxTexture: 64,
        joints: 0,
        animations: 0,
      },
    };
    const saved = saveProject({
      ...p,
      assets: (p.assets ?? []).map(a =>
        a.id === asset
          ? {
              ...a,
              status: 'done',
              versions: [...a.versions, version],
              updatedAt: version.createdAt,
            }
          : a
      ),
    });
    const last = record(
      number,
      source.last?.objects ?? [fileName(source.script).replace(/\.py$/, '')],
      source.last?.materials ?? ['palette']
    );
    of(id).set(asset, { ...source, last });
    emit(id, saved);
    return { result: 'built', asset: find(saved), project: saved, version, record: last, log: [] };
  }

  const handlers: Record<string, (args: Args) => unknown> = {
    dialog_pick_script: () => `${PIECES}/desk.py`,
    dialog_pick_files: () => [`${PIECES}/common.py`],
    built_list: ({ project: id }) => list(String(id)),
    built_add: ({ project: id, request }) => {
      const p = project(String(id));
      const next = request as NewBuilt;
      const name = next.name.trim();
      if (!name) fail('invalid', 'Give the asset a name.');
      if (!next.script.endsWith('.py'))
        fail('invalid', `${fileName(next.script)} is not a Python script.`);
      const assetId = slugify(name);
      if (
        (p.assets ?? []).some(a => a.id === assetId || a.name.toLowerCase() === name.toLowerCase())
      )
        fail('invalid', `There is already an asset called ${name}. Pick another name.`);
      const at = clock.now().toISOString();
      const asset: Asset = {
        id: assetId,
        name,
        kind: next.kind,
        stage: 'blender',
        status: 'queued',
        versions: [],
        thumbnail: null,
        createdAt: at,
        updatedAt: at,
      };
      const saved = saveProject({ ...p, assets: [...(p.assets ?? []), asset] });
      of(p.id).set(assetId, {
        script: `built/${fileName(next.script)}`,
        helpers: next.helpers.map(h => `built/${fileName(h)}`),
        root: next.root,
        last: null,
      });
      emit(p.id);
      return { asset, project: saved };
    },
    built_rebuild: ({ project: id, asset, force }) =>
      rebuild(String(id), String(asset), Boolean(force)),
  };
  return { handlers };
}
