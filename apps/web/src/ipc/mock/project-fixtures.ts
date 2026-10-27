/** The mock projects and their assets. */
import type { Asset, AssetVersion, ModelStats, ProjectManifest } from '../client';
import { clock } from '@/lib/clock';
import { HOME } from './mock-paths';

export function frostveilProject(withAssets = true): ProjectManifest {
  return {
    version: 1,
    id: 'frostveil',
    name: 'Frostveil',
    template: 'floating-diorama',
    siteRepo: `${HOME}/Documents/Projects/frostveil-site`,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 6 * 86_400_000).toISOString(),
    assets: withAssets ? frostveilAssets() : [],
  };
}

const DAY = 86_400_000;
const HOUR = 3_600_000;

function stats(triangles: number, mb: number, joints = 0, animations = 0): ModelStats {
  return { triangles, bytes: mb * 1e6, textures: 3, maxTexture: 2048, joints, animations };
}

function versions(stem: string, count: number, last: ModelStats | null): AssetVersion[] {
  return Array.from({ length: count }, (_, i) => ({
    version: i + 1,
    file: `models/${stem}_v${i + 1}.glb`,
    createdAt: new Date(clock.nowMs() - (count - i) * DAY).toISOString(),
    stats: i === count - 1 ? last : null,
  }));
}

function asset(
  partial: Partial<Asset> & Pick<Asset, 'id' | 'name' | 'kind' | 'stage' | 'status'>,
  age: number
): Asset {
  const at = new Date(clock.nowMs() - age).toISOString();
  return { versions: [], thumbnail: null, createdAt: at, updatedAt: at, ...partial };
}

/** The Board of a project a few days in. Thumbnails map to the renders in `assets/mock`. */
export function frostveilAssets(): Asset[] {
  return [
    asset(
      {
        id: 'veil-platform',
        name: 'Veil platform',
        kind: 'scene',
        stage: 'ship',
        status: 'done',
        versions: versions('veil_platform', 7, stats(412_000, 1.1)),
        thumbnail: 'thumbs/veil-platform.webp',
      },
      2 * DAY
    ),
    asset(
      {
        id: 'observatory-arch',
        name: 'Observatory arch',
        kind: 'prop',
        stage: 'web',
        status: 'cached',
        versions: versions('observatory_arch', 3, stats(38_200, 0.9)),
        thumbnail: 'thumbs/observatory-arch.webp',
      },
      3 * DAY
    ),
    asset(
      {
        id: 'elsheria',
        name: 'Elsheria',
        kind: 'character',
        stage: 'blender',
        status: 'review',
        versions: versions('elsheria', 2, stats(115_673, 1.1, 119, 2)),
        thumbnail: 'thumbs/elsheria.webp',
      },
      DAY / 3
    ),
    asset(
      {
        id: 'crystal-lantern',
        name: 'Crystal lantern',
        kind: 'prop',
        stage: 'blender',
        status: 'done',
        versions: versions('crystal_lantern', 2, stats(12_900, 0.4)),
        thumbnail: 'thumbs/crystal-lantern.webp',
      },
      4 * DAY
    ),
    asset(
      {
        id: 'veil-gate',
        name: 'Veil gate',
        kind: 'prop',
        stage: 'tripo',
        status: 'queued',
        thumbnail: 'thumbs/veil-gate.webp',
      },
      DAY
    ),
    asset(
      {
        id: 'guardian-statues',
        name: 'Guardian statues',
        kind: 'prop',
        stage: 'refs',
        status: 'review',
        thumbnail: 'thumbs/guardian-statue.webp',
      },
      DAY * 1.5
    ),
    // Sent to Refs by an older build, so it has no picture yet; opening the
    // workspace gives it the newest ref its direction sent.
    asset(
      {
        id: 'academy-landing',
        name: 'Academy landing',
        kind: 'scene',
        stage: 'refs',
        status: 'queued',
      },
      HOUR * 2
    ),
    asset(
      {
        id: 'rune-obelisk',
        name: 'Rune obelisk',
        kind: 'prop',
        stage: 'concept',
        status: 'queued',
        thumbnail: 'thumbs/rune-obelisk.webp',
      },
      DAY / 6
    ),
    asset(
      {
        id: 'frozen-angel',
        name: 'Frozen angel',
        kind: 'prop',
        stage: 'concept',
        status: 'queued',
        thumbnail: 'thumbs/frozen-angel.webp',
      },
      HOUR * 5
    ),
  ];
}

/** A Room project a week older than Frostveil, two cards in. */
export function frostCathedral(): ProjectManifest {
  return {
    version: 1,
    id: 'frost-cathedral',
    name: 'Frost cathedral',
    template: 'room',
    siteRepo: null,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 13 * DAY).toISOString(),
    assets: [
      asset(
        {
          id: 'crystal-lantern',
          name: 'Crystal lantern',
          kind: 'prop',
          stage: 'blender',
          status: 'review',
          versions: versions('crystal_lantern', 1, stats(12_900, 0.4)),
          thumbnail: 'thumbs/crystal-lantern.webp',
        },
        3 * DAY
      ),
      asset(
        {
          id: 'ice-crystals',
          name: 'Ice crystals',
          kind: 'prop',
          stage: 'refs',
          status: 'queued',
        },
        5 * DAY
      ),
    ],
  };
}

/** A Character showcase with one rigged model. */
export function elsheriaShowcase(): ProjectManifest {
  return {
    version: 1,
    id: 'elsheria-showcase',
    name: 'Elsheria showcase',
    template: 'character-showcase',
    siteRepo: `${HOME}/Documents/Projects/elsheria-showcase-site`,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 2 * DAY).toISOString(),
    assets: [
      asset(
        {
          id: 'elsheria',
          name: 'Elsheria',
          kind: 'character',
          stage: 'web',
          status: 'done',
          versions: versions('elsheria', 4, stats(115_673, 1.1, 119, 2)),
          thumbnail: 'thumbs/elsheria.webp',
        },
        DAY
      ),
    ],
  };
}

const MORE_PROJECTS: readonly [string, ProjectManifest['template']][] = [
  ['Starlit observatory', 'room'],
  ['Crypt of bells', 'room'],
  ['Frozen lake', 'floating-diorama'],
  ['Sentinel hall', 'floating-diorama'],
  ['Shore ruins', 'floating-diorama'],
];

/** `?projects=many`: enough projects that the switcher shows its search. */
export function moreProjects(): ProjectManifest[] {
  return MORE_PROJECTS.map(([name, template], i) => ({
    version: 1,
    id: name.toLowerCase().replace(/\s+/g, '-'),
    name,
    template,
    siteRepo: null,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - (20 + i * 4) * DAY).toISOString(),
    assets: [],
  }));
}

/** The projects a configured mock workspace opens with. */
export function mockProjects(many = false): ProjectManifest[] {
  return [
    frostveilProject(),
    frostCathedral(),
    elsheriaShowcase(),
    ...(many ? moreProjects() : []),
  ];
}

/** Every project id mock mode knows, so its files can be served. */
export const MOCK_PROJECT_IDS = mockProjects(true).map(p => p.id);
