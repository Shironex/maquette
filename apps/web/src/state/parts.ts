/**
 * An asset's parts: renaming, joining and hiding them in the plan
 * `prepare.json` keeps, checked with the core's rules so the Parts tab can
 * say what is wrong before a save. Plain functions over `PartsPlan`.
 */
import type { PartRow, PartsPlan } from '@/ipc/client';

export const MAX_PART_NAME = 64;

export type FullPlan = Required<PartsPlan>;

/** `plan` with every list there, as the core reads a plan with fields left out. */
export function planOf(plan: PartsPlan | null | undefined): FullPlan {
  return { names: plan?.names ?? {}, merge: plan?.merge ?? [], hidden: plan?.hidden ?? [] };
}

export function isEmptyPlan(plan: PartsPlan): boolean {
  const p = planOf(plan);
  return !Object.keys(p.names).length && !p.merge.length && !p.hidden.length;
}

/** Names `id` `name`; its own id as the name forgets the rename. */
export function renamePart(plan: PartsPlan, id: string, name: string): FullPlan {
  const p = planOf(plan);
  const names = { ...p.names };
  const trimmed = name.trim();
  if (!trimmed || trimmed === id) delete names[id];
  else names[id] = trimmed;
  return { ...p, names };
}

/** Joins `ids` into one part under the first, out of any group they were in. */
export function mergeParts(plan: PartsPlan, ids: readonly string[]): FullPlan {
  const p = planOf(plan);
  if (ids.length < 2) return p;
  const taken = new Set(ids);
  const kept = p.merge.map(g => g.filter(id => !taken.has(id))).filter(g => g.length >= 2);
  return { ...p, merge: [...kept, [...ids]] };
}

/** Undoes the join whose first part is `id`. */
export function unmergePart(plan: PartsPlan, id: string): FullPlan {
  const p = planOf(plan);
  return { ...p, merge: p.merge.filter(g => g[0] !== id) };
}

/** Hides the parts, or shows them again when every one is hidden. */
export function toggleHidden(plan: PartsPlan, ids: readonly string[]): FullPlan {
  const p = planOf(plan);
  const all = ids.every(id => p.hidden.includes(id));
  const hidden = all
    ? p.hidden.filter(id => !ids.includes(id))
    : [...p.hidden, ...ids.filter(id => !p.hidden.includes(id))];
  return { ...p, hidden };
}

/** What the core would refuse about the names, or null. */
export function planProblem(rows: readonly PartRow[], plan: PartsPlan): string | null {
  const p = planOf(plan);
  for (const name of Object.values(p.names)) {
    if (!name.trim()) return 'A part needs a name.';
    if (name.length > MAX_PART_NAME)
      return `${name.slice(0, 20)}... is over ${MAX_PART_NAME} characters.`;
  }
  const seen = new Map<string, string>();
  for (const row of rows) {
    const name = p.names[row.id] ?? row.id;
    const other = seen.get(name);
    if (other) return `${other} and ${row.id} would both ship as ${name}.`;
    seen.set(name, row.id);
  }
  return null;
}

/** How a row reads in the list: "cage", or "post + base" for a join. */
export function rowLabel(row: PartRow, rows: readonly PartRow[]): string {
  if (!row.joined.length) return row.name;
  const others = row.joined.map(id => rows.find(r => r.id === id)?.name ?? id);
  return [row.name, ...others].join(' + ');
}
