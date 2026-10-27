/**
 * The mock library's entries (`?library=demo`): models from Frostveil,
 * Frost cathedral, Elsheria showcase and a game, with their origins on the
 * projects. Frostveil's Elsheria is on v2 of an entry at v3 and Frost
 * cathedral's crystal lantern on v1 of one at v2, so both show an update.
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
    id: 'crystal-lantern',
    name: 'Crystal lantern',
    kind: 'prop',
    tags: ['props', 'lights'],
    render: 'crystal-lantern.webp',
    versions: [
      { source: 'tripo', credits: 30, from: ['frostveil', 'crystal-lantern', 1] },
      { source: 'tripo', credits: 30, from: ['frostveil', 'crystal-lantern', 2] },
    ],
    stats: stats(12_900, 0.4),
    days: 4,
  },
  {
    id: 'elsheria',
    name: 'Elsheria',
    kind: 'character',
    tags: ['characters'],
    render: 'elsheria.webp',
    versions: [
      { source: 'tripo', credits: 40, from: ['elsheria-showcase', 'elsheria', 1] },
      { source: 'tripo', credits: 40, from: ['elsheria-showcase', 'elsheria', 2] },
      { source: 'meshy', credits: 20, from: ['elsheria-showcase', 'elsheria', 4] },
    ],
    stats: stats(34_700, 1.1, 119),
    days: 1,
  },
  {
    id: 'observatory-arch',
    name: 'Observatory arch',
    kind: 'prop',
    tags: ['buildings', 'lights'],
    render: 'observatory-arch.webp',
    versions: [{ source: 'tripo', credits: 30, from: ['frostveil', 'observatory-arch', 3] }],
    stats: stats(38_200, 0.9),
    days: 3,
  },
  {
    id: 'veil-platform',
    name: 'Veil platform',
    kind: 'scene',
    tags: ['scenes'],
    render: 'veil-platform.webp',
    versions: [{ source: 'built', credits: null, from: ['frostveil', 'veil-platform', 7] }],
    stats: stats(412_000, 1.1),
    days: 2,
  },
  {
    id: 'guardian-statue',
    name: 'Guardian statue',
    kind: 'prop',
    tags: ['props', 'statues'],
    render: 'guardian-statue.webp',
    versions: [{ source: 'by-hand', credits: null, from: ['frostveil', 'guardian-statues', 1] }],
    stats: stats(8_400, 0.3),
    days: 6,
  },
  {
    id: 'veil-gate',
    name: 'Veil gate',
    kind: 'prop',
    tags: ['statues', 'gates'],
    render: 'veil-gate.webp',
    versions: [{ source: 'meshy', credits: 20, from: ['frostveil', 'veil-gate', 1] }],
    stats: stats(61_000, 1.6),
    days: 5,
  },
  {
    id: 'rune-obelisk',
    name: 'Rune obelisk',
    kind: 'prop',
    tags: ['buildings'],
    render: 'rune-obelisk.webp',
    versions: [{ source: 'tripo', credits: 30, from: ['frostveil', 'rune-obelisk', 1] }],
    stats: stats(22_700, 0.6),
    days: 7,
  },
  {
    id: 'crypt-brazier',
    name: 'Crypt brazier',
    kind: 'prop',
    tags: ['props', 'game'],
    render: 'crypt-brazier.webp',
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
  frostveil: {
    'crystal-lantern': { entry: 'crystal-lantern', version: 2 },
    elsheria: { entry: 'elsheria', version: 2 },
    'observatory-arch': { entry: 'observatory-arch', version: 1 },
    'veil-platform': { entry: 'veil-platform', version: 1 },
  },
  'frost-cathedral': { 'crystal-lantern': { entry: 'crystal-lantern', version: 1 } },
  'elsheria-showcase': { elsheria: { entry: 'elsheria', version: 3 } },
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
