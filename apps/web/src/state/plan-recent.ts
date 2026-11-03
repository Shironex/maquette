/**
 * The canvases the rail lists under Plan: the few of the open project
 * opened or edited last. When the person last had each canvas open is kept
 * on this computer, per workspace and project, so the order and the mark
 * for a canvas changed since then never leak into another project.
 */
import type { PlanSummary } from '@/ipc/client';

/** How many canvases the rail lists under Plan, as the design shows. */
export const RECENT_ON_RAIL = 3;

/** When the person last had each canvas open, in milliseconds, by canvas id. */
export type SeenAt = Readonly<Record<string, number>>;

/** One canvas on the rail. */
export interface RecentCanvas {
  id: string;
  name: string;
  /** The built in camera plan, which opens with its panel. */
  camera: boolean;
  /** Edited since the person last had it open: an agent, or a sync from another computer. */
  changed: boolean;
}

/** What the dot after a canvas on the rail says to a screen reader. */
export const CHANGED_MARK = 'changed since you last opened it';

/** Where a workspace's project keeps its seen times on this computer. */
export const seenKey = (root: string, project: string) =>
  `maquette.plan.seen.${encodeURIComponent(root)}.${encodeURIComponent(project)}`;

/** A remembered value read back as seen times, dropping anything that is not one. */
export function seenFrom(value: unknown): Record<string, number> {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value).filter(
      (entry): entry is [string, number] =>
        typeof entry[1] === 'number' && Number.isFinite(entry[1])
    )
  );
}

const editedMs = (plan: PlanSummary) => {
  const ms = Date.parse(plan.edited);
  return Number.isFinite(ms) ? ms : 0;
};

/**
 * The `count` canvases opened or edited last, newest first. A canvas that
 * does not read is left out (the list says why). One never opened here is
 * never marked: the mark is only for a change since the person looked.
 */
export function recentCanvases(
  list: readonly PlanSummary[],
  seen: SeenAt,
  count = RECENT_ON_RAIL
): RecentCanvas[] {
  return list
    .filter(p => !p.problem)
    .map(p => ({ plan: p, at: Math.max(editedMs(p), seen[p.id] ?? 0) }))
    .sort((a, b) => b.at - a.at || a.plan.id.localeCompare(b.plan.id))
    .slice(0, count)
    .map(({ plan }) => {
      const looked = seen[plan.id];
      return {
        id: plan.id,
        name: plan.name,
        camera: plan.camera,
        changed: looked !== undefined && editedMs(plan) > looked,
      };
    });
}

/** The seen times with `ids` seen at `at`, and canvases no longer listed dropped. */
export function seeing(
  seen: SeenAt,
  ids: readonly string[],
  at: number,
  listed?: readonly PlanSummary[]
): Record<string, number> {
  const next: Record<string, number> = { ...seen };
  for (const id of ids) next[id] = Math.max(next[id] ?? 0, at);
  if (!listed) return next;
  const keep = new Set(listed.map(p => p.id));
  return Object.fromEntries(Object.entries(next).filter(([id]) => keep.has(id)));
}

/** Canvases whose name changed between two reads of the list (a rename here moves `edited`). */
export function renamed(before: readonly PlanSummary[], after: readonly PlanSummary[]): string[] {
  const names = new Map(before.map(p => [p.id, p.name]));
  return after.filter(p => names.has(p.id) && names.get(p.id) !== p.name).map(p => p.id);
}
