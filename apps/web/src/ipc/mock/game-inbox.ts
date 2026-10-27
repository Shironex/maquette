/**
 * The Inbox of a game project in mock mode. Like the core, a model sent for
 * an asset checks its triangles and textures against the budget of the
 * folder that asset maps to, not the flat Tris per model setting.
 */
import type { Check, CheckStatus, FolderBudget, InboxItem, ProjectManifest } from '../client';
import { formatK } from '@/lib/format';
import { budgetFor, fileOf, gameOf } from '@/state/game';

const RANK: Record<CheckStatus, number> = { pass: 0, warning: 1, fail: 2 };

/** "the props budget", "the model folder's budget" for its top level. */
function folderLabel(budget: FolderBudget): string {
  return budget.folder ? `the ${budget.folder} budget` : "the model folder's budget";
}

/** The item's triangle and texture rows against `budget`, as the core words them. */
export function withFolderBudget(item: InboxItem, budget: FolderBudget): InboxItem {
  const report = item.report;
  if (!report) return item;
  const label = folderLabel(budget);
  const tris = report.triangles;
  const limit = budget.triangles;
  const trisStatus: CheckStatus = tris <= limit ? 'pass' : tris <= limit * 2 ? 'warning' : 'fail';
  const px = budget.texture;
  const over = report.textures
    .filter(t => Math.max(t.width ?? 0, t.height ?? 0) > px)
    .map(t => t.name);
  const checks = item.checks.map((c): Check => {
    if (c.id === 'triangles')
      return {
        ...c,
        status: trisStatus,
        detail:
          trisStatus === 'pass'
            ? `Within ${label}, ${formatK(limit)} tris.`
            : `Over ${label}, ${formatK(limit)} tris. Decimate it here, or raise the folder's budget in Inspector > Budget.`,
        fixes: trisStatus === 'pass' ? [] : [{ op: 'decimate', triangles: limit }],
      };
    if (c.id === 'textures' && report.textures.length > 0)
      return over.length === 0
        ? { ...c, status: 'pass', detail: `At or under ${label}, ${px} px.`, fixes: [] }
        : {
            ...c,
            status: 'warning',
            detail: `${over.length} ${over.length === 1 ? 'texture is' : 'textures are'} over ${label}, ${px} px: ${over.join(', ')}. Compress resizes them, or set a per model size.`,
            fixes: [{ op: 'shrink-textures', maxSize: px }],
          };
    return c;
  });
  const worst = checks.reduce<CheckStatus>(
    (w, c) => (RANK[c.status] > RANK[w] ? c.status : w),
    'pass'
  );
  return { ...item, checks, worst };
}

/**
 * The `crypt` game's Inbox: the ice crystals came back from a Tripo send
 * for Candle cluster, a prop, so they check against the props budget.
 */
export function linkGameInbox(items: InboxItem[], project: ProjectManifest | null): InboxItem[] {
  const settings = gameOf(project);
  const asset = project?.assets?.find(a => a.id === 'candle-cluster');
  if (!project || !settings || !asset) return items;
  const budget = budgetFor(settings, fileOf(settings, asset));
  if (!budget) return items;
  const link = {
    job: 'candle-cluster-1',
    project: project.id,
    asset: asset.id,
    assetName: asset.name,
    provider: 'tripo' as const,
    redoOf: null,
  };
  return items.map(item =>
    item.id === 'ice_crystals.glb' ? withFolderBudget({ ...item, link }, budget) : item
  );
}
