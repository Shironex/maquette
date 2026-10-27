import type { Asset, AssetKind, Comparison, InspectView, ProjectManifest } from '../client';
import { clock } from '@/lib/clock';
import type { Args } from './backend-options';

const DAY = 86_400_000;

/** Mixed case on purpose: older assets are capitalised, newer ones are not. */
const NAMES: Record<AssetKind, string[]> = {
  character: [
    'Hooded wanderer',
    'Lantern keeper',
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
    'Stone lanterns',
    'bamboo fence',
    'Wooden bridge',
    'prayer wheel',
    'Paper umbrella',
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
    'bonsai tree',
    'Straw hat',
    'iron kettle',
    'Wall lantern',
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
  scene: ['Island', 'courtyard', 'Harbour'],
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
    if (found.project !== 'moon-island') return found;
    const assets = (project(found.project).assets ?? []).filter(a => a.versions.length > 0);
    return { ...found, comparisons: [...found.comparisons, ...assets.map(assetComparison)] };
  };
}
