/** What the project switcher says about each project. */
import type { ProjectActivity, ProjectSummary } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { TEMPLATE_LABEL } from './onboarding';

/** `frostveil-run-25` is Run 25. */
export function runNumber(runId: string): number | null {
  const match = /-run-(\d+)$/.exec(runId);
  return match ? Number(match[1]) : null;
}

function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`;
}

/** The parts of what runs in a project: "Run 25", "2 images", "1 send". */
export function activityParts(activity: ProjectActivity): string[] {
  const parts: string[] = [];
  if (activity.run) {
    const n = runNumber(activity.run);
    parts.push(n ? `Run ${n}` : 'A run');
  }
  if (activity.concepts) parts.push(plural(activity.concepts, 'image', 'images'));
  if (activity.sends) parts.push(plural(activity.sends, 'send', 'sends'));
  return parts;
}

function joinParts(parts: readonly string[]): string {
  if (parts.length <= 1) return parts[0] ?? '';
  return `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]}`;
}

/** The short marker on a row, for example "Run 25, 2 images". Null when idle. */
export function activityLabel(activity: ProjectActivity): string | null {
  const parts = activityParts(activity);
  return parts.length ? parts.join(', ') : null;
}

/** The one line the menu shows while the open project is busy. */
export function switchNote(current: ProjectSummary | undefined): string | null {
  if (!current) return null;
  const parts = activityParts(current.activity);
  if (parts.length === 0) return null;
  const { run, concepts, sends } = current.activity;
  const one = (run ? 1 : 0) + concepts + sends === 1;
  return `${joinParts(parts)} ${one ? 'keeps' : 'keep'} going in ${current.name} while you switch.`;
}

/** "Floating diorama · 9 assets · 6 days ago" */
export function projectMeta(project: ProjectSummary, now: number): string {
  const created = Date.parse(project.createdAt);
  return [
    TEMPLATE_LABEL[project.template],
    plural(project.assetCount, 'asset', 'assets'),
    Number.isFinite(created) ? formatAgo(created, now) : null,
  ]
    .filter(Boolean)
    .join(' · ');
}

/** The shortcut that opens the switcher, as the menu shows it. */
export function switcherShortcut(mac: boolean): string {
  return mac ? '⌘P' : 'Ctrl+P';
}

/** Cmd+P on macOS, Ctrl+P elsewhere, without Shift or Alt. */
export function isSwitcherShortcut(
  e: Pick<KeyboardEvent, 'key' | 'metaKey' | 'ctrlKey' | 'shiftKey' | 'altKey'>,
  mac: boolean
): boolean {
  const mod = mac ? e.metaKey && !e.ctrlKey : e.ctrlKey && !e.metaKey;
  return mod && !e.shiftKey && !e.altKey && e.key.toLowerCase() === 'p';
}
