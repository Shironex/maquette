/**
 * Game projects in the app: their stage names, where each asset's model
 * goes in the game, its numbers against its folder's budget, and the Ship
 * plan grouped as the Ship panel shows it. The rules mirror the core's.
 */
import type {
  Asset,
  BudgetCheck,
  FolderBudget,
  GameSettings,
  ProjectManifest,
  ProjectTarget,
  ShipFile,
  ShipPlan,
  Stage,
} from '@/ipc/client';
import { formatK } from '@/lib/format';

export function isGame(project: { target?: ProjectTarget } | null | undefined): boolean {
  return project?.target === 'game';
}

/** The game settings of a game project, else null. */
export function gameOf(project: ProjectManifest | null | undefined): GameSettings | null {
  return isGame(project) ? (project?.game ?? null) : null;
}

const WEBSITE_STAGES: Record<Stage, string> = {
  concept: 'Concept',
  refs: 'Refs',
  tripo: 'Tripo',
  blender: 'Blender',
  web: 'Web',
  ship: 'Ship',
};

/** The stage names: a game exports models where a website builds its web scene. */
export function stageLabels(target: ProjectTarget | undefined): Record<Stage, string> {
  return target === 'game' ? { ...WEBSITE_STAGES, web: 'Export' } : WEBSITE_STAGES;
}

/** `props` for `props/brazier.glb`, '' at the top level. */
export function folderOf(file: string): string {
  const at = file.lastIndexOf('/');
  return at < 0 ? '' : file.slice(0, at);
}

function defaultFolder(settings: GameSettings, asset: Asset): string {
  const wanted = asset.kind === 'character' ? 'characters' : 'props';
  return settings.budgets.some(b => b.folder === wanted) ? wanted : '';
}

/** Where the asset's model goes in the model folder. */
export function fileOf(settings: GameSettings, asset: Asset): string {
  const recorded = settings.models[asset.id];
  if (recorded) return recorded.file;
  const name = `${asset.id.replaceAll('-', '_')}.${settings.format}`;
  const folder = defaultFolder(settings, asset);
  return folder ? `${folder}/${name}` : name;
}

/** The budget of the file's folder, or of its nearest parent. */
export function budgetFor(settings: GameSettings, file: string): FolderBudget | null {
  let folder = folderOf(file);
  for (;;) {
    const found = settings.budgets.find(b => b.folder === folder);
    if (found) return found;
    if (!folder) return null;
    folder = folderOf(folder);
  }
}

/** The asset's newest model against its folder's budget, when both are known. */
export function checkAsset(settings: GameSettings, asset: Asset): BudgetCheck | null {
  const stats = asset.versions[asset.versions.length - 1]?.stats;
  const budget = stats ? budgetFor(settings, fileOf(settings, asset)) : null;
  if (!stats || !budget) return null;
  const problems: string[] = [];
  if (stats.triangles > budget.triangles)
    problems.push(`Over ${formatK(budget.triangles)} triangles`);
  if (stats.maxTexture > budget.texture) problems.push(`Texture over ${budget.texture} px`);
  return {
    folder: budget.folder,
    triangles: stats.triangles,
    maxTriangles: budget.triangles,
    texture: stats.maxTexture,
    maxTexture: budget.texture,
    rigged: stats.joints > 0,
    problems,
  };
}

/** The asset is over its folder's budget. */
export function isOver(settings: GameSettings, asset: Asset): boolean {
  return (checkAsset(settings, asset)?.problems.length ?? 0) > 0;
}

export interface BudgetSummary {
  models: number;
  over: Asset[];
  /** How many of the over models are over their folder's triangles. */
  overTriangles: number;
  /** How many are over their folder's texture size (a model can be both). */
  overTextures: number;
}

/** How many models the folder budgets check, which are over, and on what. */
export function budgetSummary(project: ProjectManifest | null | undefined): BudgetSummary | null {
  const settings = gameOf(project);
  if (!settings || !project) return null;
  const checks = (project.assets ?? []).flatMap(asset => {
    const check = checkAsset(settings, asset);
    return check ? [{ asset, check }] : [];
  });
  const over = checks.filter(c => c.check.problems.length > 0);
  return {
    models: checks.length,
    over: over.map(c => c.asset),
    overTriangles: over.filter(c => c.check.triangles > c.check.maxTriangles).length,
    overTextures: over.filter(c => c.check.texture > c.check.maxTexture).length,
  };
}

/** "14.6k / 12k", the triangles against the budget. */
export function trianglesLine(check: BudgetCheck): string {
  return `${formatK(check.triangles)} / ${formatK(check.maxTriangles)}`;
}

/** Why a rigged model over its triangles is not decimated. */
export const BY_HAND_REASON =
  'Rigged: reduce by hand. Decimate would tear the skin weights and shape keys.';

/** The model is over its triangles but rigged, so Decimate is not offered. */
export function needsHand(check: BudgetCheck): boolean {
  return check.rigged && check.triangles > check.maxTriangles;
}

/**
 * The fix-ups "Add the fix-ups" gives the model, as the core's
 * `fixups_for`: Decimate (never on a rigged model) and Shrink textures.
 */
export function autoFixes(check: BudgetCheck): ('Decimate' | 'Shrink')[] {
  const out: ('Decimate' | 'Shrink')[] = [];
  if (check.triangles > check.maxTriangles && !check.rigged) out.push('Decimate');
  if (check.texture > check.maxTexture) out.push('Shrink');
  return out;
}

/** What brings the model within its budget, as the list names it. */
export function fixName(check: BudgetCheck): string | null {
  if (needsHand(check)) return 'By hand';
  return autoFixes(check)[0] ?? null;
}

export interface ShipGroups {
  added: ShipFile[];
  replaced: ShipFile[];
  unchanged: ShipFile[];
  /** Exports Ship leaves alone because a hand made file is there. */
  blocked: ShipFile[];
  handMade: ShipFile[];
}

/** The Ship plan grouped the way the panel lists it. */
export function shipGroups(plan: ShipPlan): ShipGroups {
  const of = (change: ShipFile['change']) =>
    plan.files.filter(f => f.change === change && !f.blocked);
  return {
    added: of('new'),
    replaced: of('replaced'),
    unchanged: of('unchanged'),
    blocked: plan.files.filter(f => f.blocked),
    handMade: plan.handMade,
  };
}

/** The replaced files that are still the game's own imported originals. */
export function importedOriginals(groups: ShipGroups): ShipFile[] {
  return groups.replaced.filter(f => f.imported);
}

/** "2 replace the imported originals", or null when none do. */
export function originalsLine(groups: ShipGroups): string | null {
  const n = importedOriginals(groups).length;
  if (!n) return null;
  return n === 1 ? '1 replaces the imported original' : `${n} replace the imported originals`;
}

/** The plain line about committing first, when the game repo has changes. */
export function commitFirstLine(plan: ShipPlan): string | null {
  const n = plan.uncommitted ?? 0;
  if (!n) return null;
  const files = n === 1 ? '1 file' : `${n} files`;
  return `The game repo has ${files} not committed yet. Commit them first, so the Ship change is easy to review and undo.`;
}

/** "3 new · 3 replaced · 0 removed". */
export function shipCounts(groups: ShipGroups): string {
  return `${groups.added.length} new · ${groups.replaced.length} replaced · 0 removed`;
}
