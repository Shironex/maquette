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
    problems,
  };
}

export interface BudgetSummary {
  models: number;
  over: Asset[];
}

/** How many models the budgets check and which are over. */
export function budgetSummary(project: ProjectManifest | null | undefined): BudgetSummary | null {
  const settings = gameOf(project);
  if (!settings || !project) return null;
  const checked = (project.assets ?? []).filter(a => checkAsset(settings, a));
  return {
    models: checked.length,
    over: checked.filter(a => (checkAsset(settings, a)?.problems.length ?? 0) > 0),
  };
}

/** "14.6k / 12k", the triangles against the budget. */
export function trianglesLine(check: BudgetCheck): string {
  return `${formatK(check.triangles)} / ${formatK(check.maxTriangles)}`;
}

/** The fix-up that brings the model within its budget, as the list names it. */
export function fixName(check: BudgetCheck): string | null {
  if (check.triangles > check.maxTriangles) return 'Decimate';
  if (check.texture > check.maxTexture) return 'Shrink';
  return null;
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

/** "3 new · 3 replaced · 0 removed". */
export function shipCounts(groups: ShipGroups): string {
  return `${groups.added.length} new · ${groups.replaced.length} replaced · 0 removed`;
}
