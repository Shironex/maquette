import type { ReactNode } from 'react';
import type { StatusBarItem, StatusDot } from '@maquette/ui';
import type {
  CodexUsage,
  ConceptJob,
  InboxSnapshot,
  ProviderJob,
  RunRecord,
  ServerStatus,
  ToolReport,
  WorkspaceSnapshot,
} from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { usageLabel, usageTone, type UsageTone } from '@/state/codex-usage';
import { isWaiting } from '@/state/lab';
import { serverHost } from '@/state/page';
import { runStatusLine, runSummary } from '@/state/pipeline';
import type { ViewId } from '@/state/views';

/** Everything the shell's status bar reads, gathered from the stores. */
export interface ShellStatus {
  view: ViewId;
  workspace: WorkspaceSnapshot | null;
  reports: readonly ToolReport[];
  checking: boolean;
  labJobs: readonly ConceptJob[];
  inbox: InboxSnapshot | null;
  latest: RunRecord | undefined;
  now: number;
  mcp: StatusBarItem | null;
  /** "Rendering 7 of 13 files" while a render runs. */
  rendering?: string | null;
  /** "Rig doctor: pose test 9 of 13" while a case runs. */
  rigging?: string | null;
  sends: readonly ProviderJob[];
  /** The site dev server the Page view and Preview on site share. */
  server?: ServerStatus | null;
  /** Draws the site dev server's item as a control (its menu). */
  siteControl?: (label: string, dot: StatusDot) => ReactNode;
  /** How many models the library holds, once loaded. */
  libraryModels?: number | null;
  /** How many clips the animation library holds, and where, once loaded. */
  animationClips?: number | null;
  animationsDir?: string | null;
  /** The last Codex usage read. */
  codexUsage?: CodexUsage | null;
  /** Draws the Codex usage item as a control (a read on click, the resets in its tooltip). */
  codexControl?: (usage: CodexUsage) => ReactNode;
}

/** Codex CLI's state in the Lab; a known usage read says it is signed in already. */
function codexItems(
  view: ViewId,
  codex: ToolReport | undefined,
  usage: CodexUsage | null | undefined
): StatusBarItem[] {
  if (view !== 'lab' || !codex) return [];
  if (codex.status === 'found' && usage?.known) return [];
  return [
    codex.status === 'found'
      ? { label: 'Codex CLI, signed in', icon: 'terminal-window' }
      : { label: 'Codex CLI not ready', dot: 'offline' },
  ];
}

const USAGE_DOT: Record<UsageTone, StatusDot> = { ok: 'ok', warn: 'running', limit: 'offline' };

/** "Codex 16% (5h) · 3% (week)", in the Lab and while its images generate. */
function codexUsageItems(s: ShellStatus): StatusBarItem[] {
  const usage = s.codexUsage;
  const live = s.labJobs.some(
    j => j.state !== 'done' && j.state !== 'failed' && j.state !== 'cancelled'
  );
  if (!usage || (s.view !== 'lab' && !live)) return [];
  const item: StatusBarItem = { label: usageLabel(usage), dot: USAGE_DOT[usageTone(usage)] };
  const node = s.codexControl?.(usage);
  return [node ? { ...item, node } : item];
}

function labItems(jobs: readonly ConceptJob[]): StatusBarItem[] {
  const generating = jobs.filter(j => j.state === 'running').length;
  const waiting = jobs.filter(isWaiting).length;
  if (!generating && !waiting) return [];
  return [
    {
      label: generating
        ? `${generating} ${generating === 1 ? 'image' : 'images'} generating${waiting ? `, ${waiting} waiting` : ''}`
        : `${waiting} ${waiting === 1 ? 'image' : 'images'} waiting`,
      dot: 'running',
    },
  ];
}

function blenderItem(checking: boolean, blender: ToolReport | undefined): StatusBarItem {
  if (checking) return { label: 'Checking tools', dot: 'running' };
  return blender?.status === 'found'
    ? { label: `Blender ${blender.version ?? ''}, headless`.replace('  ', ' '), icon: 'cube' }
    : { label: 'Blender not found', dot: 'offline' };
}

function inboxItems(
  inbox: InboxSnapshot | null,
  workspace: WorkspaceSnapshot | null
): StatusBarItem[] {
  const inboxDir =
    inbox?.dir ?? (workspace ? `${workspace.display}/${workspace.config.layout.inbox}` : null);
  if (!inboxDir) return [];
  return [
    {
      label: `Inbox at ${inboxDir}${inbox?.watching ? ', watching' : ''}`,
      ...(inbox?.watching ? { dot: 'ok' as const } : { icon: 'tray' as const }),
    },
  ];
}

function libraryItems(s: ShellStatus): StatusBarItem[] {
  if (s.view !== 'library' || !s.workspace || s.libraryModels == null) return [];
  const n = s.libraryModels;
  return [
    {
      label: `Library at ${s.workspace.display}/library, ${n} ${n === 1 ? 'model' : 'models'}`,
      dot: 'ok',
    },
  ];
}

function animationItems(s: ShellStatus): StatusBarItem[] {
  if (s.view !== 'animations' || s.animationClips == null || !s.animationsDir) return [];
  const n = s.animationClips;
  const dir = s.workspace
    ? s.animationsDir.replace(s.workspace.root, s.workspace.display)
    : s.animationsDir;
  return [{ label: `Animations at ${dir}, ${n} ${n === 1 ? 'clip' : 'clips'}`, dot: 'ok' }];
}

function runItems(latest: RunRecord | undefined, now: number): StatusBarItem[] {
  if (!latest) return [{ label: 'No runs yet', right: true }];
  if (latest.state === 'running') {
    return [
      { label: runStatusLine(latest), dot: 'running' },
      { label: runSummary(latest), right: true },
    ];
  }
  return [
    ...(latest.state === 'failed'
      ? [{ label: runStatusLine(latest), dot: 'offline' as const }]
      : latest.notShipped
        ? [{ label: runStatusLine(latest), dot: 'warn' as const }]
        : []),
    {
      label: `Last run ${formatAgo(latest.finishedAtMs ?? latest.queuedAtMs ?? now, now)}`,
      right: true,
    },
  ];
}

function sendItems(sends: readonly ProviderJob[]): StatusBarItem[] {
  const sending = sends.filter(j => j.state === 'running' || j.state === 'queued').length;
  const handWaiting = sends.filter(j => j.state === 'waiting').length;
  const sendParts = [
    sending ? `${sending} ${sending === 1 ? 'model' : 'models'} on the way` : null,
    handWaiting ? `${handWaiting} by hand` : null,
  ].filter(Boolean);
  if (!sendParts.length) return [];
  return [
    sending
      ? { label: sendParts.join(', '), dot: 'running' }
      : { label: `${handWaiting} waiting for your model`, icon: 'hand-pointing' },
  ];
}

function pageItem(server: ServerStatus | null | undefined): StatusBarItem | null {
  if (!server) return null;
  if (server.state === 'running' && server.url) {
    return { label: `Site dev page on ${serverHost(server)}`, dot: 'ok' };
  }
  if (server.state === 'starting') return { label: 'Site dev page starting', dot: 'running' };
  if (server.state === 'failed') return { label: 'Site dev page did not start', dot: 'offline' };
  return null;
}

function pageItems(s: ShellStatus): StatusBarItem[] {
  const item = pageItem(s.server);
  if (!item) return [];
  const node = s.siteControl?.(item.label, item.dot ?? 'idle');
  return [node ? { ...item, node } : item];
}

/** The status bar line, left to right: worker, tools, agents, Codex usage, inbox, library, Lab, sends, the site dev page, then the run. */
export function shellStatusItems(s: ShellStatus): StatusBarItem[] {
  return [
    { label: 'Local worker connected', dot: 'ok' },
    blenderItem(
      s.checking,
      s.reports.find(r => r.id === 'blender')
    ),
    ...(s.mcp ? [s.mcp] : []),
    ...codexItems(
      s.view,
      s.reports.find(r => r.id === 'codex'),
      s.codexUsage
    ),
    ...codexUsageItems(s),
    ...(s.view === 'lab' ? [] : inboxItems(s.inbox, s.workspace)),
    ...(s.rendering ? [{ label: s.rendering, dot: 'running' as const }] : []),
    ...(s.rigging ? [{ label: s.rigging, dot: 'running' as const }] : []),
    ...libraryItems(s),
    ...animationItems(s),
    ...labItems(s.labJobs),
    ...sendItems(s.sends),
    ...pageItems(s),
    ...runItems(s.latest, s.now),
  ];
}
