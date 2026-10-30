/**
 * Pure helpers for going through the inbox many models at a time: the
 * groups the table shows (what each model needs, worst last), the filter
 * chips, and what accepting a selection does before it runs.
 */
import type { InboxItem, ProviderJob } from '@/ipc/client';
import { goesTo, sortChecks } from './inbox';
import { PROVIDER_LABEL } from './providers';

/** What a model needs: accept it, look at it, throw it away, or re-export it. */
export type TriageGroup = 'pass' | 'review' | 'replaced' | 'unreadable';

/** The list filter: every model, or one group. */
export type TriageFilter = 'all' | TriageGroup;

/** The groups in the order the table shows them, worst last. */
export const GROUPS: readonly { id: TriageGroup; label: string }[] = [
  { id: 'pass', label: 'Checks pass' },
  { id: 'review', label: 'To review' },
  { id: 'replaced', label: 'Replaced by a redo' },
  { id: 'unreadable', label: 'Unreadable' },
];

/** The group a model sits in. A replaced model goes there whatever its checks say. */
export function triageGroup(item: InboxItem): TriageGroup {
  if (item.state === 'failed') return 'unreadable';
  if (item.replacedBy != null) return 'replaced';
  return item.worst === 'pass' ? 'pass' : 'review';
}

/** How many models each group holds, plus `all`. */
export function groupCounts(items: readonly InboxItem[]): Record<TriageFilter, number> {
  const counts = { all: items.length, pass: 0, review: 0, replaced: 0, unreadable: 0 };
  for (const item of items) counts[triageGroup(item)] += 1;
  return counts;
}

/** The models a filter keeps, in their order. */
export function filterItems(items: readonly InboxItem[], filter: TriageFilter): InboxItem[] {
  return filter === 'all' ? [...items] : items.filter(i => triageGroup(i) === filter);
}

/** The non empty groups with their models, in the table's order. */
export function groupItems(
  items: readonly InboxItem[]
): { id: TriageGroup; label: string; items: InboxItem[] }[] {
  return GROUPS.map(g => ({ ...g, items: items.filter(i => triageGroup(i) === g.id) })).filter(
    g => g.items.length > 0
  );
}

/** The table's Checks cell: what stands out, in a few words. */
export function checksCell(item: InboxItem): string {
  if (item.state === 'failed') return 'could not read it';
  if (item.replacedBy != null) return `v${item.replacedBy} accepted`;
  if (item.worst === 'pass') return 'pass';
  return sortChecks(item.checks).find(c => c.status !== 'pass')?.title ?? 'to review';
}

/** "v3" for the version Accept makes, "new" for a model that goes to no asset yet. */
export function versionCell(item: InboxItem): string {
  if (item.nextVersion != null) return `v${item.nextVersion}`;
  return goesTo(item) ? 'v1' : 'new';
}

/** Where the model came from: its send's provider, else "Download" or "Dropped in". */
export function fromCell(item: InboxItem): string {
  if (item.link) return PROVIDER_LABEL[item.link.provider];
  return item.aim ? 'Download' : 'Dropped in';
}

/** How the model found its asset: an API send, a By hand send by name, an attach, or not at all. */
export function matchedCell(item: InboxItem): string {
  if (item.link) return item.link.provider === 'by-hand' ? 'name' : 'API send';
  return item.aim ? 'attached' : 'none';
}

/** What accepting the selected models does, counted before it runs. */
export interface AcceptPlan {
  /** The ids that can be accepted. */
  ids: string[];
  /** How many become the first version of a model. */
  fresh: number;
  /** "Ice crystals v2" for each that becomes a later version. */
  later: string[];
  /** By hand sends that close as done once their asset gets a model. */
  handoffs: number;
  /** How many of the selection cannot be accepted (unreadable files). */
  skipped: number;
}

/** The plan for accepting `checked`, with the By hand sends still waiting in `jobs`. */
export function acceptPlan(
  items: readonly InboxItem[],
  checked: readonly string[],
  jobs: readonly ProviderJob[]
): AcceptPlan {
  const picked = items.filter(i => checked.includes(i.id));
  const ok = picked.filter(i => i.state !== 'failed');
  const later = ok
    .filter(i => (i.nextVersion ?? 1) > 1)
    .map(i => `${goesTo(i)?.assetName ?? i.name} v${i.nextVersion}`);
  const goals = new Set(ok.map(goesTo).flatMap(g => (g ? [`${g.project}/${g.asset}`] : [])));
  const handoffs = jobs.filter(
    j =>
      j.provider === 'by-hand' &&
      (j.state === 'waiting' || j.state === 'inbox') &&
      goals.has(`${j.project}/${j.asset}`)
  ).length;
  return {
    ids: ok.map(i => i.id),
    fresh: ok.length - later.length,
    later,
    handoffs,
    skipped: picked.length - ok.length,
  };
}

/** The line next to the selection count: "all checks pass", "2 to review". */
export function selectionNote(items: readonly InboxItem[], checked: readonly string[]): string {
  const picked = items.filter(i => checked.includes(i.id));
  if (picked.length === 0) return '';
  const counts = groupCounts(picked);
  if (counts.pass === picked.length) return 'all checks pass';
  const parts = [
    counts.review ? `${counts.review} to review` : null,
    counts.replaced ? `${counts.replaced} replaced` : null,
    counts.unreadable ? `${counts.unreadable} unreadable` : null,
  ].filter(Boolean);
  return parts.join(', ');
}
