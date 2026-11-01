/**
 * Game templates in mock mode. The list and the rules mirror the core's
 * `settings::game::templates` and `game::templates`.
 */
import type {
  FolderBudget,
  GameExport,
  GameFormat,
  GameSettings,
  GameTemplate,
  GameTemplateId,
  GameTemplatePreview,
  ProjectManifest,
  TemplateNewPath,
} from '../client';
import { fileOf, isOver } from '@/state/game';
import { repoModels } from './game-fixtures';

const PROPS = { folder: 'props', triangles: 12_000, texture: 1024 };
const CHARACTERS = { folder: 'characters', triangles: 60_000, texture: 2048 };
const KIT = { folder: 'kit', triangles: 5_000, texture: 1024 };
const row = (folder: string, triangles: number, texture: number): FolderBudget => ({
  folder,
  triangles,
  texture,
});

const JUDGMENT = 'The per model numbers are judgment, not published limits.';

type Tuned = Pick<GameTemplate, 'id' | 'name' | 'detail' | 'group' | 'why' | 'budgets'> & {
  textures: 'png' | 'jpg';
  textureCap: number;
  dropMaps?: boolean;
};

const shape = (t: Pick<GameTemplate, 'id' | 'name' | 'detail' | 'why' | 'format' | 'budgets'>) =>
  ({ ...t, group: 'shape', textures: null, textureCap: null, dropMaps: null }) as GameTemplate;
const tuned = ({ dropMaps = false, ...t }: Tuned): GameTemplate => ({
  ...t,
  format: 'glb',
  dropMaps,
});

export const GAME_TEMPLATES: GameTemplate[] = [
  shape({
    id: 'prop-set',
    name: 'Prop set',
    detail: 'One file per prop, no scene',
    why: 'A .glb holds a prop and its textures in one file. The budget is what a new game gives props/.',
    format: 'glb',
    budgets: [PROPS],
  }),
  shape({
    id: 'character',
    name: 'Character',
    detail: 'One rigged model with its clips',
    why: 'Rigs and clips only export with a .glb. The budget is what a new game gives characters/.',
    format: 'glb',
    budgets: [CHARACTERS],
  }),
  shape({
    id: 'level-kit',
    name: 'Level kit',
    detail: 'Modular pieces that repeat, as .glb',
    why: 'A piece is drawn many times in a view, so it gets 5k triangles against the 12k of a prop.',
    format: 'glb',
    budgets: [KIT],
  }),
  shape({
    id: 'level-kit-obj',
    name: 'Level kit, .obj',
    detail: 'The same pieces for an engine that reads .obj',
    why: 'Static pieces need no rig or clips, so .obj with its .mtl carries all of them. Same budget as Level kit.',
    format: 'obj',
    budgets: [KIT],
  }),
  tuned({
    id: 'web-game',
    group: 'target',
    name: 'Web game',
    detail: 'Small downloads for a browser game',
    why: `Browsers download every model and texture, so everything is small: JPG textures (no transparency, use Keep for cutouts) capped at 1024 px. Game export writes plain files and no KTX2 or meshopt, so compress afterwards if you need to. No published limit applies here. ${JUDGMENT}`,
    textures: 'jpg',
    textureCap: 1024,
    budgets: [row('props', 10_000, 1024), row('characters', 25_000, 1024), row('kit', 3_000, 512)],
  }),
  tuned({
    id: 'godot',
    group: 'engine',
    name: 'Godot',
    detail: 'A .glb project with room for PNG textures',
    why: `Godot imports .glb files directly. PNG textures capped at 2048 px, with an environment/ folder for scenes. No published limit applies here. ${JUDGMENT}`,
    textures: 'png',
    textureCap: 2048,
    budgets: [
      row('props', 20_000, 2048),
      row('characters', 50_000, 2048),
      row('environment', 30_000, 2048),
    ],
  }),
  tuned({
    id: 'unity',
    group: 'engine',
    name: 'Unity',
    detail: 'Mid-range numbers for a desktop or console build',
    why: `PNG textures capped at 2048 px, 1024 px on props and kit pieces. Check that your Unity project reads .glb before you ship. No published limit applies here. ${JUDGMENT}`,
    textures: 'png',
    textureCap: 2048,
    budgets: [row('props', 15_000, 1024), row('characters', 40_000, 2048), row('kit', 5_000, 1024)],
  }),
  tuned({
    id: 'unreal',
    group: 'engine',
    name: 'Unreal',
    detail: 'High numbers for a desktop or console build',
    why: `PNG textures capped at 4096 px, an environment/ folder for scenes. Check that your Unreal project imports .glb before you ship. No published limit applies here. ${JUDGMENT}`,
    textures: 'png',
    textureCap: 4096,
    budgets: [
      row('props', 40_000, 2048),
      row('characters', 80_000, 4096),
      row('environment', 60_000, 2048),
    ],
  }),
  tuned({
    id: 'mobile',
    group: 'target',
    name: 'Mobile',
    detail: 'Small models and 512 px textures for phones',
    why: `Phones have little memory and bandwidth, so models and textures stay small: JPG textures capped at 1024 px, 512 px on props and kit pieces. No published limit applies here. ${JUDGMENT}`,
    textures: 'jpg',
    textureCap: 1024,
    budgets: [row('props', 4_000, 512), row('characters', 15_000, 1024), row('kit', 2_000, 512)],
  }),
  tuned({
    id: 'vr-standalone',
    group: 'target',
    name: 'VR standalone',
    detail: 'Headsets that render on their own chip',
    why: "A standalone headset draws the scene twice and has a small budget. Meta's device comparison page lists whole scene limits of about 1.5M triangles and 200 draw calls on Quest 3 and 750K and 100 on Quest 2. That is a limit for the whole view, not a model. The per model rows here are judgment, picked so a view of a few dozen models stays under it. JPG textures capped at 1024 px.",
    textures: 'jpg',
    textureCap: 1024,
    budgets: [row('props', 5_000, 1024), row('characters', 20_000, 1024), row('kit', 2_000, 512)],
  }),
  tuned({
    id: 'game-jam',
    group: 'target',
    name: 'Game jam',
    detail: 'One folder, small files, no extra maps',
    why: `For a few days of work: every model in the model folder checks against one budget of 20k triangles and 1024 px, JPG textures, and normal and metal-roughness maps left out so files stay small. A folder with its own budget keeps it. No published limit applies here. ${JUDGMENT}`,
    textures: 'jpg',
    textureCap: 1024,
    dropMaps: true,
    budgets: [row('', 20_000, 1024)],
  }),
];

/** The export settings after `template`: what it names, over what the game has. */
export const exportAfter = (current: GameExport, t: GameTemplate): GameExport => ({
  textures: t.textures ?? current.textures,
  textureCap: t.textureCap ?? current.textureCap,
  dropMaps: t.dropMaps ?? current.dropMaps,
});

export function merged(current: FolderBudget[], rows: FolderBudget[]): FolderBudget[] {
  const out = [...current];
  for (const row of rows) {
    const at = out.findIndex(b => b.folder === row.folder);
    if (at >= 0) out[at] = row;
    else out.push(row);
  }
  return out;
}

export const templateOf = (id: GameTemplateId) => GAME_TEMPLATES.find(t => t.id === id)!;

/** `to`'s file name in the folder of `from`. */
function sameFolder(from: string, to: string): string {
  const name = to.slice(to.lastIndexOf('/') + 1);
  const at = from.lastIndexOf('/');
  return at < 0 ? name : `${from.slice(0, at)}/${name}`;
}

export function templateOutcome(
  project: ProjectManifest,
  settings: GameSettings,
  id: GameTemplateId,
  shipped: Set<string>
) {
  const template = templateOf(id);
  const next: GameSettings = {
    ...settings,
    format: template.format,
    export: exportAfter(settings.export, template),
    budgets: merged(settings.budgets, template.budgets),
    models: { ...settings.models },
  };
  const inGame = new Set([...shipped, ...repoModels(settings.format).map(m => m.file)]);
  const keptFiles: string[] = [];
  const newPaths: TemplateNewPath[] = [];
  for (const asset of project.assets ?? []) {
    if (!asset.versions.length) continue;
    const [from, to] = [fileOf(settings, asset), fileOf(next, asset)];
    if (from === to) continue;
    if (inGame.has(from)) {
      next.models[asset.id] = { file: from, importedVersion: null };
      keptFiles.push(from);
      continue;
    }
    const stays = sameFolder(from, to);
    if (stays !== to) next.models[asset.id] = { file: stays, importedVersion: null };
    if (stays !== from) newPaths.push({ asset: asset.id, from, to: stays });
  }
  return { next, keptFiles, newPaths };
}

const extOf = (file: string): GameFormat => (file.endsWith('.obj') ? 'obj' : 'glb');

export function templatePreview(
  project: ProjectManifest,
  settings: GameSettings,
  id: GameTemplateId,
  shipped: Set<string>
): GameTemplatePreview {
  const template = templateOf(id);
  const { next, keptFiles, newPaths } = templateOutcome(project, settings, id, shipped);
  const budgets = template.budgets.flatMap(after => {
    const before = settings.budgets.find(b => b.folder === after.folder) ?? null;
    const same = before?.triangles === after.triangles && before?.texture === after.texture;
    return same ? [] : [{ folder: after.folder, before, after }];
  });
  const notes: string[] = [];
  const files = repoModels(settings.format);
  const others = files.filter(m => extOf(m.file) !== template.format).length;
  if (template.format !== settings.format && others * 2 > files.length) {
    notes.push(
      `Most of the ${files.length} models in the model folder are not .${template.format}. Check that the engine reads it.`
    );
  }
  if (
    template.budgets.some(b => b.folder === 'kit') &&
    settings.budgets.some(b => b.folder === 'props')
  ) {
    notes.push(
      'This game has a budget for props/, so new models still go there. Move a piece to kit/ with its Game folder in the Budget tab.'
    );
  }
  if (template.dropMaps && !settings.export.dropMaps) {
    notes.push(
      "Normal and metal-roughness maps are left out of exported models. Check that the game's shaders do not read them."
    );
  }
  if (template.textures === 'jpg' && settings.export.textures !== 'jpg') {
    notes.push(
      'JPG textures have no transparency. Set the texture format to Keep, or PNG, for a model with cutouts.'
    );
  }
  const own = settings.budgets.filter(b => b.folder !== '').map(b => `${b.folder}/`);
  if (template.budgets.every(b => b.folder === '') && own.length) {
    notes.push(
      `The row for the top level of the model folder is new. This game has budgets for ${own.join(', ')}, and a folder with its own row keeps it.`
    );
  }
  const assets = project.assets ?? [];
  const after = exportAfter(settings.export, template);
  return {
    template,
    formatBefore: settings.format,
    formatAfter: template.format,
    exportBefore: settings.export,
    exportAfter: after,
    budgets,
    keptFiles,
    newPaths,
    overBefore: assets.filter(a => isOver(settings, a)).length,
    overAfter: assets.filter(a => isOver(next, a)).length,
    notes,
    changes:
      settings.format !== template.format ||
      JSON.stringify(settings.export) !== JSON.stringify(after) ||
      budgets.length > 0,
  };
}
