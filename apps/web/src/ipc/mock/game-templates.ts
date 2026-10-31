/**
 * Game templates in mock mode. The list and the rules mirror the core's
 * `settings::game::templates` and `game::templates`.
 */
import type {
  FolderBudget,
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

export const GAME_TEMPLATES: GameTemplate[] = [
  {
    id: 'prop-set',
    name: 'Prop set',
    detail: 'One file per prop, no scene',
    why: 'A .glb holds a prop and its textures in one file. The budget is what a new game gives props/.',
    format: 'glb',
    budgets: [PROPS],
  },
  {
    id: 'character',
    name: 'Character',
    detail: 'One rigged model with its clips',
    why: 'Rigs and clips only export with a .glb. The budget is what a new game gives characters/.',
    format: 'glb',
    budgets: [CHARACTERS],
  },
  {
    id: 'level-kit',
    name: 'Level kit',
    detail: 'Modular pieces that repeat, as .glb',
    why: 'A piece is drawn many times in a view, so it gets 5k triangles against the 12k of a prop.',
    format: 'glb',
    budgets: [KIT],
  },
  {
    id: 'level-kit-obj',
    name: 'Level kit, .obj',
    detail: 'The same pieces for an engine that reads .obj',
    why: 'Static pieces need no rig or clips, so .obj with its .mtl carries all of them. Same budget as Level kit.',
    format: 'obj',
    budgets: [KIT],
  },
];

function merged(current: FolderBudget[], rows: FolderBudget[]): FolderBudget[] {
  const out = [...current];
  for (const row of rows) {
    const at = out.findIndex(b => b.folder === row.folder);
    if (at >= 0) out[at] = row;
    else out.push(row);
  }
  return out;
}

const templateOf = (id: GameTemplateId) => GAME_TEMPLATES.find(t => t.id === id)!;

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
  const assets = project.assets ?? [];
  return {
    template,
    formatBefore: settings.format,
    formatAfter: template.format,
    budgets,
    keptFiles,
    newPaths,
    overBefore: assets.filter(a => isOver(settings, a)).length,
    overAfter: assets.filter(a => isOver(next, a)).length,
    notes,
    changes: settings.format !== template.format || budgets.length > 0,
  };
}
