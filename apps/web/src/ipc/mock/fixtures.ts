/**
 * Data for mock mode. Neutral names and paths only, so screenshots never
 * point at a real project on someone's disk.
 */
import type {
  Asset,
  AssetVersion,
  ModelStats,
  ProjectManifest,
  ToolReport,
  WorkspaceConfig,
} from '../client';

export const HOME = '/Users/maker';
export const DEFAULT_ROOT = '~/Maquette';
export const SITE_REPO = '~/Documents/Projects/moon-island-site';

export function defaultConfig(): WorkspaceConfig {
  return {
    version: 1,
    layout: { projects: 'projects', inbox: 'inbox', cache: 'cache' },
    tripo: {
      model: 'H3.1',
      hd: true,
      topology: 'triangle',
      texture: '2K',
      removeLighting: true,
      pbr: true,
      character: { quads: 50000, multiView: true, autoRig: true, rigPreset: 'mixamo' },
    },
    budgets: { sceneMb: 3, trisPerModel: 60000 },
    compression: { textureCodec: 'etc1s', meshopt: 'medium' },
    agents: {
      mcpEnabled: false,
      mcpPort: 4791,
      rules: { blenderHeadlessOnly: true, neverCaptureScreen: true, askBeforeShipping: true },
    },
    currentProject: null,
  };
}

function report(partial: Partial<ToolReport> & Pick<ToolReport, 'id' | 'label'>): ToolReport {
  return {
    required: false,
    status: 'found',
    source: 'path',
    path: null,
    version: null,
    detail: null,
    hint: null,
    warning: null,
    ...partial,
  };
}

export function toolReports(scenario: string): ToolReport[] {
  const blender =
    scenario === 'no-blender'
      ? report({
          id: 'blender',
          label: 'Blender',
          required: true,
          status: 'missing',
          source: null,
          detail: 'not found on PATH',
          hint: 'Install Blender from blender.org, or locate Blender.app.',
        })
      : report({
          id: 'blender',
          label: 'Blender',
          required: true,
          source: 'known-location',
          path: '/Applications/Blender.app/Contents/MacOS/Blender',
          version: '4.3.2',
          detail: 'runs headless',
        });
  return [
    blender,
    report({
      id: 'codex',
      label: 'Codex CLI',
      path: '/opt/homebrew/bin/codex',
      version: '0.46.0',
      detail: 'signed in',
    }),
    report({ id: 'ffmpeg', label: 'ffmpeg', path: '/opt/homebrew/bin/ffmpeg', version: '7.1' }),
    report({
      id: 'toktx',
      label: 'KTX-Software toktx',
      status: 'missing',
      source: null,
      detail: 'not found on PATH',
      hint: 'Install KTX-Software, then Re-check. Until then Compress keeps textures as PNG.',
    }),
    report({
      id: 'gltf-transform',
      label: 'gltf-transform',
      source: 'npx',
      path: '/opt/homebrew/bin/npx',
      detail: 'via npx, @gltf-transform/cli',
    }),
    report({ id: 'node', label: 'Node', path: '/opt/homebrew/bin/node', version: '22.23.0' }),
  ];
}

export function moonIsland(withAssets = true): ProjectManifest {
  return {
    version: 1,
    id: 'moon-island',
    name: 'Moon Island',
    template: 'floating-diorama',
    siteRepo: `${HOME}/Documents/Projects/moon-island-site`,
    sourceFolder: null,
    createdAt: new Date(Date.now() - 6 * 86_400_000).toISOString(),
    assets: withAssets ? moonAssets() : [],
  };
}

const DAY = 86_400_000;

function stats(triangles: number, mb: number, joints = 0, animations = 0): ModelStats {
  return { triangles, bytes: mb * 1e6, textures: 3, maxTexture: 2048, joints, animations };
}

function versions(stem: string, count: number, last: ModelStats | null): AssetVersion[] {
  return Array.from({ length: count }, (_, i) => ({
    version: i + 1,
    file: `models/${stem}_v${i + 1}.glb`,
    createdAt: new Date(Date.now() - (count - i) * DAY).toISOString(),
    stats: i === count - 1 ? last : null,
  }));
}

function asset(
  partial: Partial<Asset> & Pick<Asset, 'id' | 'name' | 'kind' | 'stage' | 'status'>,
  age: number
): Asset {
  const at = new Date(Date.now() - age).toISOString();
  return { versions: [], thumbnail: null, createdAt: at, updatedAt: at, ...partial };
}

/** The Board of a project a few days in. Thumbnails map to bundled images. */
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
        thumbnail: 'thumbs/island.jpg',
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
        thumbnail: 'thumbs/shop-house.jpg',
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
        thumbnail: 'thumbs/character.jpg',
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
      },
      4 * DAY
    ),
    asset({ id: 'sakura', name: 'Sakura', kind: 'prop', stage: 'tripo', status: 'queued' }, DAY),
    asset(
      { id: 'lanterns', name: 'Stone lanterns', kind: 'prop', stage: 'refs', status: 'review' },
      DAY * 1.5
    ),
    asset(
      { id: 'far-spire', name: 'Far spire', kind: 'prop', stage: 'concept', status: 'queued' },
      DAY / 6
    ),
  ];
}
