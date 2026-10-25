/** The mock projects and their assets. */
import type { Asset, AssetVersion, ModelStats, ProjectManifest } from '../client';
import { clock } from '@/lib/clock';
import { HOME } from './mock-paths';

export function moonIsland(withAssets = true): ProjectManifest {
  return {
    version: 1,
    id: 'moon-island',
    name: 'Moon Island',
    template: 'floating-diorama',
    siteRepo: `${HOME}/Documents/Projects/moon-island-site`,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 6 * 86_400_000).toISOString(),
    assets: withAssets ? moonAssets() : [],
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
export function moonAssets(): Asset[] {
  return [
    asset(
      {
        id: 'island',
        name: 'Island',
        kind: 'scene',
        stage: 'web',
        status: 'done',
        versions: versions('island', 7, stats(412_000, 1.1)),
        thumbnail: 'thumbs/island.webp',
      },
      2 * DAY
    ),
    asset(
      {
        id: 'shop-house',
        name: 'Shop house',
        kind: 'prop',
        stage: 'web',
        status: 'cached',
        versions: versions('shop_house', 3, stats(38_200, 0.9)),
        thumbnail: 'thumbs/shop-house.webp',
      },
      3 * DAY
    ),
    asset(
      {
        id: 'character',
        name: 'Character',
        kind: 'character',
        stage: 'blender',
        status: 'review',
        versions: versions('character', 2, stats(94_612, 1.4, 65, 2)),
        thumbnail: 'thumbs/character.webp',
      },
      DAY / 3
    ),
    asset(
      {
        id: 'vending',
        name: 'Vending machine',
        kind: 'prop',
        stage: 'blender',
        status: 'done',
        versions: versions('vending', 2, stats(12_900, 0.4)),
        thumbnail: 'thumbs/vending-machine.webp',
      },
      4 * DAY
    ),
    asset(
      {
        id: 'sakura',
        name: 'Sakura',
        kind: 'prop',
        stage: 'tripo',
        status: 'queued',
        thumbnail: 'thumbs/sakura-large.webp',
      },
      DAY
    ),
    asset(
      {
        id: 'lanterns',
        name: 'Stone lanterns',
        kind: 'prop',
        stage: 'refs',
        status: 'review',
        thumbnail: 'thumbs/stone-lantern.webp',
      },
      DAY * 1.5
    ),
    // Sent to Refs by an older build, so it has no picture yet; opening the
    // workspace gives it the newest ref its direction sent.
    asset(
      {
        id: 'maquette-landing-studio',
        name: 'Maquette landing studio',
        kind: 'scene',
        stage: 'refs',
        status: 'queued',
      },
      HOUR * 2
    ),
    asset(
      {
        id: 'far-spire',
        name: 'Far spire',
        kind: 'prop',
        stage: 'concept',
        status: 'queued',
        thumbnail: 'thumbs/far-spire.webp',
      },
      DAY / 6
    ),
  ];
}

/** A Room project a week older than Moon Island, two cards in. */
export function rooftopCinema(): ProjectManifest {
  return {
    version: 1,
    id: 'rooftop-cinema',
    name: 'Rooftop cinema',
    template: 'room',
    siteRepo: null,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 13 * DAY).toISOString(),
    assets: [
      asset(
        {
          id: 'vending',
          name: 'Vending machine',
          kind: 'prop',
          stage: 'blender',
          status: 'review',
          versions: versions('vending', 1, stats(12_900, 0.4)),
          thumbnail: 'thumbs/vending-machine.webp',
        },
        3 * DAY
      ),
      asset(
        {
          id: 'flower-shrubs',
          name: 'Flower shrubs',
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
export function lanternKeeper(): ProjectManifest {
  return {
    version: 1,
    id: 'lantern-keeper',
    name: 'Lantern keeper',
    template: 'character-showcase',
    siteRepo: `${HOME}/Documents/Projects/lantern-keeper-site`,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 2 * DAY).toISOString(),
    assets: [
      asset(
        {
          id: 'character',
          name: 'Character',
          kind: 'character',
          stage: 'web',
          status: 'done',
          versions: versions('character', 4, stats(94_612, 1.4, 65, 2)),
          thumbnail: 'thumbs/character.webp',
        },
        DAY
      ),
    ],
  };
}

const MORE_PROJECTS: readonly [string, ProjectManifest['template']][] = [
  ['Late train', 'room'],
  ['Kotatsu den', 'room'],
  ['Night market', 'floating-diorama'],
  ['Harbour lights', 'floating-diorama'],
  ['Stone garden', 'floating-diorama'],
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
  return [moonIsland(), rooftopCinema(), lanternKeeper(), ...(many ? moreProjects() : [])];
}

/** Every project id mock mode knows, so its files can be served. */
export const MOCK_PROJECT_IDS = mockProjects(true).map(p => p.id);
