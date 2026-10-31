import type {
  Asset,
  AssetKind,
  Comparison,
  InspectModel,
  InspectView,
  ProjectManifest,
} from '../client';
import { clock } from '@/lib/clock';
import type { Args } from './backend-options';

const DAY = 86_400_000;

/** Mixed case on purpose: older assets are capitalised, newer ones are not. */
const NAMES: Record<AssetKind, string[]> = {
  character: [
    'Hooded wanderer',
    'Ice warden',
    'fox spirit',
    'Stone guardian',
    'tea house cat',
    'Old fisherman',
    'crow messenger',
    'Moth priestess',
  ],
  prop: [
    'Brazier',
    'armillary sphere',
    'Guardian statues',
    'bamboo fence',
    'Wooden bridge',
    'prayer wheel',
    'Frost banner',
    'koi pond',
    'Shrine gate',
    'bronze bell',
    'Tea set',
    'hanging scroll',
    'Sake barrel',
    'moss rock',
    'Wind chime',
    'cedar stump',
    'Offering box',
    'candle stand',
    'Fishing net',
    'rope coil',
    'Clay jar',
    'frozen pine',
    'Straw hat',
    'iron kettle',
    'Wall sconce',
    'rice sack',
    'Stepping stones',
    'cherry branch',
    'Fire pit',
    'cart wheel',
    'Bench',
    'water bucket',
    'Torii fragment',
    'mossy statue',
    'Hay bale',
    'oil lamp',
    'Garden shears',
    'lotus pad',
  ],
  scene: ['Platform', 'courtyard', 'Harbour'],
};

function slug(name: string): string {
  return name.toLowerCase().replaceAll(' ', '-');
}

/** 49 assets across the three kinds, with one to four versions each. */
export function manyAssets(): Asset[] {
  const at = new Date(clock.nowMs() - 3 * DAY).toISOString();
  return (Object.keys(NAMES) as AssetKind[]).flatMap(kind =>
    NAMES[kind].map((name, i) => {
      const id = `many-${slug(name)}`;
      const count = 1 + ((i + name.length) % 4);
      return {
        id,
        name,
        kind,
        stage: 'web',
        status: 'done',
        versions: Array.from({ length: count }, (_, v) => ({
          version: v + 1,
          file: `models/${id}_v${v + 1}.glb`,
          createdAt: at,
          stats: null,
        })),
        thumbnail: null,
        createdAt: at,
        updatedAt: at,
      } satisfies Asset;
    })
  );
}

/** The project with its many assets added, as `?inspect=many` shows them. */
export function withManyAssets(project: ProjectManifest): ProjectManifest {
  return { ...project, assets: [...(project.assets ?? []), ...manyAssets()] };
}

/** How many models `?inspect=diorama` puts in the scene, as a busy diorama has. */
export const DIORAMA_MODELS = 66;

/** The project filled up to a diorama's models, as `?inspect=diorama` shows it. */
export function withDioramaAssets(project: ProjectManifest): ProjectManifest {
  const many = withManyAssets(project);
  const assets = many.assets ?? [];
  const models = assets.filter(a => a.versions.length > 0).length;
  const filler = manyAssets()[0]!;
  const extra = Array.from({ length: Math.max(0, DIORAMA_MODELS - models) }, (_, i) => {
    const id = `diorama-piece-${i + 1}`;
    const version = { ...filler.versions[0]!, file: `models/${id}_v1.glb` };
    return { ...filler, id, name: `Diorama piece ${i + 1}`, versions: [version] };
  });
  return { ...many, assets: [...assets, ...extra] };
}

/** Frostveil with the assets `?inspect=` asks for. */
export function withInspectAssets(project: ProjectManifest, state: string | null): ProjectManifest {
  if (project.id !== 'frostveil') return project;
  if (state === 'many') return withManyAssets(project);
  return state === 'diorama' ? withDioramaAssets(project) : project;
}

/** Project files to the bundled model that stands in for each, for `?inspect=diorama`. */
export function dioramaFiles(): Record<string, string> {
  const empty = { assets: [] } as unknown as ProjectManifest;
  const versions = (withDioramaAssets(empty).assets ?? []).flatMap(a => a.versions);
  return Object.fromEntries(versions.map(v => [v.file, 'ice_crystals.glb']));
}

function assetComparison(asset: Asset): Comparison {
  const last = asset.versions[asset.versions.length - 1]?.version;
  return {
    id: `asset/${asset.id}`,
    label: asset.name,
    kind: 'asset',
    sides: asset.versions.map(v => ({
      id: `asset/${asset.id}/v${v.version}/side`,
      label: `v${v.version}`,
      note: v.version === last ? 'current' : v.createdAt.slice(0, 10),
      members: [{ model: `asset/${asset.id}/v${v.version}`, position: [0, 0, 0] }],
    })),
  };
}

/** Gives the inspect view one comparison per versioned asset, as the core does. */
export function wrapManyInspect(
  all: Record<string, (args: Args) => unknown>,
  project: (id: string) => ProjectManifest
) {
  const inspect = all.inspect_project;
  if (!inspect) return;
  all.inspect_project = args => {
    const found = inspect(args) as InspectView;
    if (found.project !== 'frostveil') return found;
    const assets = (project(found.project).assets ?? []).filter(a => a.versions.length > 0);
    return { ...found, comparisons: [...found.comparisons, ...assets.map(assetComparison)] };
  };
}

/** A version of the frost lantern as the view lists it, for `?parts=`. */
function lanternModel(base: InspectModel, version: number): InspectModel {
  const tripo = version <= 2;
  return {
    ...base,
    id: `asset/frost-lantern/v${version}`,
    model: 'frost_lantern',
    label: `Frost lantern v${version}`,
    source: 'version',
    file: `${base.file.split('/web/dist/')[0]}/models/frost_lantern_v${version}.glb`,
    path: `models/frost_lantern_v${version}.glb`,
    asset: 'frost-lantern',
    version,
    tweakAsset: 'frost-lantern',
    materials: 4,
    materialNames: ['stone', 'iron', 'crystal_glow', 'brass'],
    triangles: tripo ? 18_200 : 9_600,
    bytes: tripo ? 1_100_000 : 620_000,
    gpuBytes: tripo ? 9_400_000 : 5_100_000,
    size: tripo ? [0.34, 0.59, 0.34] : [0.32, 0.54, 0.32],
    error: null,
  };
}

/** The frost lantern's versions, placed in the scene, for every `?parts=`. */
function wrapCompareInspect(
  all: Record<string, (args: Args) => unknown>,
  project: (id: string) => ProjectManifest
) {
  const inspect = all.inspect_project;
  if (!inspect) return;
  all.inspect_project = args => {
    const found = inspect(args) as InspectView;
    const asset = project(found.project).assets?.find(a => a.id === 'frost-lantern');
    const base = found.models[0];
    if (found.project !== 'frostveil' || !asset || !base) return found;
    // It stands where a stop looks, so the stop can judge it.
    const place = found.stops.stops[2]?.target ?? [0, 0, 0];
    const shipped: InspectModel = {
      ...base,
      id: 'shipped/frost_lantern',
      model: 'frost_lantern',
      asset: 'frost-lantern',
      tweakAsset: 'frost-lantern',
    };
    const scene = found.comparisons.map(c =>
      c.kind !== 'scene'
        ? c
        : {
            ...c,
            sides: c.sides.map(s => ({
              ...s,
              members: [...s.members, { model: shipped.id, position: place }],
            })),
          }
    ) as InspectView['comparisons'];
    return {
      ...found,
      models: [...found.models, shipped, ...asset.versions.map(v => lanternModel(base, v.version))],
      comparisons: [...scene, { ...assetComparison(asset), label: asset.name }],
    };
  };
}

/** The inspect fixtures the mock's options ask for. */
export function wrapInspectFixtures(
  all: Record<string, (args: Args) => unknown>,
  project: (id: string) => ProjectManifest,
  inspect: string | null,
  parts: string | null
) {
  if (inspect === 'many') wrapManyInspect(all, project);
  if (parts) wrapCompareInspect(all, project);
}
