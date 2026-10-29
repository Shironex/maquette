/**
 * Mock game repos and game projects: Crypt game, a CMake game whose .glb
 * models sit in props/, enemies/ and characters/, and Maze game, whose
 * models are .obj with their .mtl. The numbers mirror what the core's scan
 * would find.
 */
import type {
  Asset,
  FolderBudget,
  GameFormat,
  GameRepoInspection,
  GameSettings,
  ModelStats,
  ProjectManifest,
  ScannedModel,
} from '../client';
import { clock } from '@/lib/clock';
import { HOME } from './mock-paths';

export const CRYPT_REPO = `${HOME}/Documents/Projects/crypt-game`;
export const MAZE_REPO = `${HOME}/Documents/Projects/maze-game`;

export interface RepoModel {
  file: string;
  triangles: number;
  textures: number;
  maxTexture: number;
  mb: number;
  joints?: number;
  clips?: number;
}

const CRYPT_MODELS: readonly RepoModel[] = [
  { file: 'props/altar.glb', triangles: 9_800, textures: 2, maxTexture: 1024, mb: 2.6 },
  { file: 'props/banner.glb', triangles: 2_100, textures: 2, maxTexture: 2048, mb: 3.1 },
  { file: 'props/bone_pile.glb', triangles: 6_400, textures: 2, maxTexture: 1024, mb: 1.9 },
  { file: 'props/brazier.glb', triangles: 3_200, textures: 2, maxTexture: 1024, mb: 1.4 },
  { file: 'props/candle_stand.glb', triangles: 1_800, textures: 1, maxTexture: 1024, mb: 0.8 },
  { file: 'props/chain.glb', triangles: 900, textures: 1, maxTexture: 512, mb: 0.3 },
  { file: 'props/chest.glb', triangles: 5_200, textures: 2, maxTexture: 1024, mb: 1.7 },
  { file: 'props/coffin.glb', triangles: 4_100, textures: 2, maxTexture: 1024, mb: 1.6 },
  { file: 'props/iron_gate.glb', triangles: 14_600, textures: 2, maxTexture: 2048, mb: 2.2 },
  { file: 'props/skull_lamp.glb', triangles: 2_600, textures: 2, maxTexture: 1024, mb: 1.1 },
  { file: 'props/torch.glb', triangles: 1_200, textures: 1, maxTexture: 512, mb: 0.4 },
  {
    file: 'enemies/bat.glb',
    triangles: 6_000,
    textures: 1,
    maxTexture: 1024,
    mb: 1.3,
    joints: 18,
    clips: 2,
  },
  {
    file: 'enemies/ghoul.glb',
    triangles: 22_000,
    textures: 2,
    maxTexture: 2048,
    mb: 4.4,
    joints: 41,
    clips: 3,
  },
  {
    file: 'enemies/knight.glb',
    triangles: 46_200,
    textures: 3,
    maxTexture: 2048,
    mb: 6.8,
    joints: 52,
    clips: 4,
  },
  {
    file: 'enemies/wraith.glb',
    triangles: 18_000,
    textures: 2,
    maxTexture: 2048,
    mb: 3.9,
    joints: 36,
    clips: 3,
  },
  {
    file: 'characters/heroine.glb',
    triangles: 58_000,
    textures: 4,
    maxTexture: 2048,
    mb: 7.3,
    joints: 65,
    clips: 6,
  },
];

/** A hand rigged heroine with a long clip list, for `?game=hero`. */
export const HERO_MODEL: RepoModel = {
  file: 'characters/heroine.glb',
  triangles: 115_700,
  textures: 1,
  maxTexture: 2048,
  mb: 15,
  joints: 119,
  clips: 187,
};

const MAZE_NAMES = [
  'stile',
  'gate',
  'gate_lantern',
  'hedge_corner',
  'hedge_straight',
  'hedge_end',
  'bench',
  'well',
  'fountain',
  'statue',
  'arch',
  'lamp_post',
  'crate',
  'barrel',
  'cart',
  'sign',
  'fence',
  'fence_post',
  'rock_small',
  'rock_large',
  'tree_oak',
  'tree_pine',
  'bush',
  'flower_bed',
  'path_stone',
  'stairs',
  'bridge',
  'pillar',
  'urn',
  'torch_wall',
  'chest',
  'key',
  'door',
  'shrine',
  'banner',
];

const MAZE_MODELS: readonly RepoModel[] = MAZE_NAMES.map((name, i) => ({
  file: `${name}.obj`,
  triangles: name === 'gate_lantern' ? 24_500 : 1_200 + ((i * 1_377) % 9_000),
  textures: 2,
  maxTexture: 1024,
  mb: 0.4 + ((i * 7) % 12) / 10,
}));

export function repoModels(format: GameFormat): readonly RepoModel[] {
  return format === 'obj' ? MAZE_MODELS : CRYPT_MODELS;
}

/** The core's defaults for these folders (`game::default_budgets`). */
export function cryptBudgets(): FolderBudget[] {
  return [
    { folder: 'props', triangles: 12_000, texture: 1024 },
    { folder: 'characters', triangles: 60_000, texture: 2048 },
    { folder: 'enemies', triangles: 40_000, texture: 2048 },
  ];
}

export function mazeBudgets(): FolderBudget[] {
  return [{ folder: '', triangles: 20_000, texture: 1024 }];
}

/** What New project finds in a repo: Crypt game, Maze game, or no game. */
export function inspectGameRepo(path: string): GameRepoInspection {
  const obj = /maze/i.test(path);
  const game = obj || /game|crypt/i.test(path);
  const models = game ? repoModels(obj ? 'obj' : 'glb') : [];
  return {
    path,
    exists: game || /site|landing|web/i.test(path),
    isGit: game,
    branch: game ? 'main' : null,
    cmake: game,
    targets: !game ? [] : obj ? ['maze'] : ['crypt', 'crypt_tests'],
    modelsDir: game ? 'assets/models' : null,
    format: game ? (obj ? 'obj' : 'glb') : null,
    glbFiles: obj ? 0 : models.length,
    objFiles: obj ? models.length : 0,
    folders: !game ? [] : obj ? [''] : ['props', 'characters', 'enemies'],
    loader: !game ? null : obj ? 'tinyobjloader' : 'cgltf',
    readsKtx2: false,
    readsMeshopt: false,
  };
}

export function statsOf(model: RepoModel): ModelStats {
  return {
    triangles: model.triangles,
    bytes: model.mb * 1e6,
    textures: model.textures,
    maxTexture: model.maxTexture,
    joints: model.joints ?? 0,
    animations: model.clips ?? 0,
  };
}

/** `props/iron_gate.glb` is the asset `iron-gate`, named Iron gate. */
export function assetIdOf(file: string): string {
  const stem =
    file
      .split('/')
      .pop()
      ?.replace(/\.(glb|obj)$/, '') ?? file;
  return stem.replaceAll('_', '-');
}

function nameOf(id: string): string {
  const words = id.replaceAll('-', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

const DAY = 86_400_000;

/** The asset an import makes of a repo file: its v1 is the repo file. */
export function importedAsset(model: RepoModel, overBudget: boolean): Asset {
  const id = assetIdOf(model.file);
  const at = new Date(clock.nowMs() - 2 * DAY).toISOString();
  const ext = model.file.endsWith('.obj') ? 'obj' : 'glb';
  const file = ext === 'obj' ? `models/${id}_v1/${id}.obj` : `models/${id}_v1.glb`;
  return {
    id,
    name: nameOf(id),
    kind: model.joints ? 'character' : 'prop',
    stage: overBudget ? 'blender' : 'ship',
    status: overBudget ? 'review' : 'done',
    versions: [{ version: 1, file, createdAt: at, stats: statsOf(model) }],
    thumbnail: null,
    createdAt: at,
    updatedAt: at,
  };
}

export function scanned(
  model: RepoModel,
  settings: GameSettings,
  asset: string | null
): ScannedModel {
  const at = model.file.lastIndexOf('/');
  const folder = at < 0 ? '' : model.file.slice(0, at);
  const budget = settings.budgets.find(b => b.folder === folder) ?? settings.budgets[0];
  const problems = budget
    ? [
        ...(model.triangles > budget.triangles
          ? [`Over ${budget.triangles / 1000}k triangles`]
          : []),
        ...(model.maxTexture > budget.texture ? [`Texture over ${budget.texture} px`] : []),
      ]
    : [];
  return {
    file: model.file,
    folder,
    format: model.file.endsWith('.obj') ? 'obj' : 'glb',
    stats: statsOf(model),
    error: null,
    budget: budget
      ? {
          folder: budget.folder,
          triangles: model.triangles,
          maxTriangles: budget.triangles,
          texture: model.maxTexture,
          maxTexture: budget.texture,
          rigged: (model.joints ?? 0) > 0,
          problems,
        }
      : null,
    asset,
  };
}

/** A game project, before or after its models were imported. */
export function gameProject(kind: 'crypt' | 'maze', imported: boolean): ProjectManifest {
  const obj = kind === 'maze';
  const settings: GameSettings = {
    format: obj ? 'obj' : 'glb',
    modelsDir: 'assets/models',
    export: { textures: 'png', textureCap: 1024, dropMaps: false },
    bake: false,
    budgets: obj ? mazeBudgets() : cryptBudgets(),
    models: {},
  };
  const picked = repoModels(settings.format).filter(m => !m.file.startsWith('characters/'));
  const assets = imported
    ? picked.map(m => importedAsset(m, scanned(m, settings, null).budget?.problems.length !== 0))
    : [];
  if (imported) {
    for (const m of picked) {
      settings.models[assetIdOf(m.file)] = { file: m.file, importedVersion: 1 };
    }
  }
  return {
    version: 1,
    id: obj ? 'maze-game' : 'crypt-game',
    name: obj ? 'Maze game' : 'Crypt game',
    template: 'game-import',
    target: 'game',
    game: settings,
    siteRepo: obj ? MAZE_REPO : CRYPT_REPO,
    sourceFolder: null,
    createdAt: new Date(clock.nowMs() - 3 * DAY).toISOString(),
    assets,
  };
}
