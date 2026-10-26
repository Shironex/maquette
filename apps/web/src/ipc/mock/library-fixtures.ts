/**
 * The mock library's entries (`?library=demo`): models from Moon Island,
 * Rooftop cinema, Lantern keeper and a game, with their origins on the
 * projects. Moon Island's character is on v2 of an entry at v3 and Rooftop
 * cinema's vending machine on v1 of one at v2, so both show an update.
 */
import type {
  AssetKind,
  AssetOrigin,
  LibraryEntry,
  LibrarySource,
  LibraryVersion,
  ModelStats,
} from '../client';
import { clock } from '@/lib/clock';

const DAY = 86_400_000;

function stats(triangles: number, mb: number, joints = 0): ModelStats {
  return { triangles, bytes: mb * 1e6, textures: 3, maxTexture: 2048, joints, animations: 0 };
}

interface Seed {
  id: string;
  name: string;
  kind: AssetKind;
  tags: string[];
  /** The render under `assets/mock` its picture shows, if any. */
  render?: string;
  ext?: 'glb' | 'obj';
  /** Oldest first: what made each version, its credits and where it came from. */
  versions: { source: LibrarySource; credits: number | null; from: [string, string, number] }[];
  stats: ModelStats;
  days: number;
}

const SEEDS: readonly Seed[] = [
  {
    id: 'vending-machine',
    name: 'Vending machine',
    kind: 'prop',
    tags: ['props', 'street'],
    render: 'vending-machine.webp',
    versions: [
      { source: 'tripo', credits: 30, from: ['moon-island', 'vending', 1] },
      { source: 'tripo', credits: 30, from: ['moon-island', 'vending', 2] },
    ],
    stats: stats(12_900, 0.4),
    days: 4,
  },
  {
    id: 'character',
    name: 'Character',
    kind: 'character',
    tags: ['characters'],
    render: 'character.webp',
    versions: [
      { source: 'tripo', credits: 40, from: ['lantern-keeper', 'character', 1] },
      { source: 'tripo', credits: 40, from: ['lantern-keeper', 'character', 2] },
      { source: 'meshy', credits: 20, from: ['lantern-keeper', 'character', 4] },
    ],
    stats: stats(48_200, 1.2, 65),
    days: 1,
  },
  {
    id: 'shop-house',
    name: 'Shop house',
    kind: 'prop',
    tags: ['buildings', 'street'],
    render: 'shop-house.webp',
    versions: [{ source: 'tripo', credits: 30, from: ['moon-island', 'shop-house', 3] }],
    stats: stats(38_200, 0.9),
    days: 3,
  },
  {
    id: 'island',
    name: 'Island',
    kind: 'scene',
    tags: ['scenes'],
    render: 'island.webp',
    versions: [{ source: 'built', credits: null, from: ['moon-island', 'island', 7] }],
    stats: stats(412_000, 1.1),
    days: 2,
  },
  {
    id: 'stone-lantern',
    name: 'Stone lantern',
    kind: 'prop',
    tags: ['props', 'garden'],
    render: 'stone-lantern.webp',
    versions: [{ source: 'by-hand', credits: null, from: ['moon-island', 'lanterns', 1] }],
    stats: stats(8_400, 0.3),
    days: 6,
  },
  {
    id: 'sakura-tree',
    name: 'Sakura tree',
    kind: 'prop',
    tags: ['garden', 'trees'],
    render: 'sakura-large.webp',
    versions: [{ source: 'meshy', credits: 20, from: ['moon-island', 'sakura', 1] }],
    stats: stats(61_000, 1.6),
    days: 5,
  },
  {
    id: 'far-spire',
    name: 'Far spire',
    kind: 'prop',
    tags: ['buildings'],
    render: 'far-spire.webp',
    versions: [{ source: 'tripo', credits: 30, from: ['moon-island', 'far-spire', 1] }],
    stats: stats(22_700, 0.6),
    days: 7,
  },
  {
    id: 'crypt-brazier',
    name: 'Crypt brazier',
    kind: 'prop',
    tags: ['props', 'game'],
    ext: 'obj',
    versions: [{ source: 'imported', credits: null, from: ['crypt-game', 'brazier', 1] }],
    stats: stats(3_100, 0.2),
    days: 8,
  },
];

function version(seed: Seed, i: number): LibraryVersion {
  const n = i + 1;
  const key = seed.id.replace(/-/g, '_');
  const made = seed.versions[i]!;
  const file = seed.ext === 'obj' ? `models/${key}_v${n}/${key}.obj` : `models/${key}_v${n}.glb`;
  return {
    version: n,
    file,
    createdAt: new Date(clock.nowMs() - (seed.days + seed.versions.length - n) * DAY).toISOString(),
    stats:
      n === seed.versions.length ? seed.stats : stats(Math.round(seed.stats.triangles * 1.4), 1.5),
    source: made.source,
    credits: made.credits,
    hash: `${seed.id}-${n}`,
    from: { project: made.from[0], asset: made.from[1], version: made.from[2] },
    thumbnail: seed.render ? `thumbs/${key}_v${n}.webp` : null,
    prep: null,
    materials: null,
  };
}

export function libraryEntries(): LibraryEntry[] {
  return SEEDS.map(seed => {
    const versions = seed.versions.map((_, i) => version(seed, i));
    return {
      version: 1,
      id: seed.id,
      name: seed.name,
      kind: seed.kind,
      tags: seed.tags,
      refs: [],
      versions,
      createdAt: versions[0]!.createdAt,
      updatedAt: versions.at(-1)!.createdAt,
    };
  });
}

/** Which project assets came from which entry and version. */
export const LIBRARY_ORIGINS: Record<string, Record<string, AssetOrigin>> = {
  'moon-island': {
    vending: { entry: 'vending-machine', version: 2 },
    character: { entry: 'character', version: 2 },
    'shop-house': { entry: 'shop-house', version: 1 },
    island: { entry: 'island', version: 1 },
  },
  'rooftop-cinema': { vending: { entry: 'vending-machine', version: 1 } },
  'lantern-keeper': { character: { entry: 'character', version: 3 } },
};

/** Each entry picture's path under the library folder, to the render it shows. */
export function libraryFiles(): Record<string, string> {
  const out: Record<string, string> = {};
  for (const seed of SEEDS) {
    if (!seed.render) continue;
    seed.versions.forEach((_, i) => {
      out[`${seed.id}/thumbs/${seed.id.replace(/-/g, '_')}_v${i + 1}.webp`] = seed.render!;
    });
  }
  return out;
}
