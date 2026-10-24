/** Pure helpers for the model inbox. */
import type { Status } from '@maquette/ui';
import type { Check, InboxItem } from '@/ipc/client';
import { formatK } from '@/lib/format';

/** The chip a list row shows. */
export function itemChip(item: InboxItem): { status: Status; label: string } {
  if (item.state === 'failed') return { status: 'failed', label: 'unreadable' };
  if (item.worst === 'pass') return { status: 'done', label: 'checks passed' };
  return { status: 'review', label: 'needs review' };
}

/** "94.6k tris · 3 textures". */
export function itemMeta(item: InboxItem): string {
  const r = item.report;
  if (!r) return item.error ?? 'Could not read this file';
  const textures = `${r.textures.length} ${r.textures.length === 1 ? 'texture' : 'textures'}`;
  return `${formatK(r.triangles)} tris · ${textures}`;
}

/** "2 passed · 3 to review". */
export function checkSummary(checks: readonly Check[]): string {
  const passed = checks.filter(c => c.status === 'pass').length;
  const review = checks.length - passed;
  if (checks.length === 0) return 'No checks';
  if (review === 0) return `${passed} passed`;
  return `${passed} passed · ${review} to review`;
}

/** The sentence over the action bar. */
export function reviewPrompt(item: InboxItem): string {
  if (item.state === 'failed') return 'This file could not be read. Re-export it, then Reprocess.';
  const fails = item.checks.filter(c => c.status === 'fail').length;
  const warns = item.checks.filter(c => c.status === 'warning').length;
  if (fails + warns === 0) return 'Every check passed. Accept it to move it into the project.';
  const parts = [
    fails ? `${fails} ${fails === 1 ? 'failure' : 'failures'}` : null,
    warns ? `${warns} ${warns === 1 ? 'warning' : 'warnings'}` : null,
  ].filter(Boolean);
  return `Review the ${parts.join(' and ')} before sending this model to the build.`;
}

/** Fail first, then warnings, then passes, keeping the order within each. */
export function sortChecks(checks: readonly Check[]): Check[] {
  const rank = { fail: 0, warning: 1, pass: 2 } as const;
  return [...checks].sort((a, b) => rank[a.status] - rank[b.status]);
}

/** The row to open first: the newest readable model, else the newest file. */
export function firstToReview(items: readonly InboxItem[]): InboxItem | null {
  return items.find(i => i.state !== 'failed') ?? items[0] ?? null;
}
