import type { StatusBarItem } from '@maquette/ui';
import type {
  ConceptJob,
  InboxSnapshot,
  ProviderJob,
  RunRecord,
  ServerStatus,
  ToolReport,
  WorkspaceSnapshot,
} from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { shortAddress } from '@/state/page';
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
  sends: readonly ProviderJob[];
  /** The site dev server the Page view and Preview on site share. */
  server?: ServerStatus | null;
  /** How many models the library holds, once loaded. */
  libraryModels?: number | null;
}

function codexItems(view: ViewId, codex: ToolReport | undefined): StatusBarItem[] {
  if (view !== 'lab' || !codex) return [];
  return [
    codex.status === 'found'
      ? { label: 'Codex CLI, signed in', icon: 'terminal-window' }
      : { label: 'Codex CLI not ready', dot: 'offline' },
  ];
}

function labItems(jobs: readonly ConceptJob[]): StatusBarItem[] {
  const generating = jobs.filter(j => j.state === 'running').length;
  const waiting = jobs.filter(j => j.state === 'queued' || j.state === 'rate-limited').length;
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

function pageItems(server: ServerStatus | null | undefined): StatusBarItem[] {
  if (!server) return [];
  if (server.state === 'running' && server.url) {
    const host = shortAddress(server.url).replace(/\/$/, '');
    return [{ label: `Site dev page on ${host}`, dot: 'ok' }];
  }
  if (server.state === 'starting') return [{ label: 'Site dev page starting', dot: 'running' }];
  if (server.state === 'failed') return [{ label: 'Site dev page did not start', dot: 'offline' }];
  return [];
}

/** The status bar line, left to right: worker, tools, agents, inbox, library, Lab, sends, the site dev page, then the run. */
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
      s.reports.find(r => r.id === 'codex')
    ),
    ...(s.view === 'lab' ? [] : inboxItems(s.inbox, s.workspace)),
    ...libraryItems(s),
    ...labItems(s.labJobs),
    ...sendItems(s.sends),
    ...pageItems(s.server),
    ...runItems(s.latest, s.now),
  ];
}
