import type {
  AnimationsView,
  InboxItem,
  LabView,
  Ledger,
  LibraryView,
  LogEntry,
  McpStatus,
  PipelineView,
  ProjectManifest,
} from '../client';
import type { Args } from './backend-options';

/**
 * `?stress=lists`: every long list at the size a busy workspace reaches, to
 * measure the views with. The copies reuse the fixtures' pictures and files.
 */
export const STRESS = {
  assets: 500,
  library: 300,
  clips: 500,
  inbox: 200,
  directions: 60,
  logLines: 5000,
  spends: 1000,
  calls: 200,
} as const;

/** `count` items made from `from` in turn, each with its copy number. */
function copies<T>(from: readonly T[], count: number, make: (item: T, n: number) => T): T[] {
  if (from.length === 0) return [];
  return Array.from({ length: count }, (_, n) => make(from[n % from.length]!, n + 1));
}

/** Frostveil with 500 assets copied from its own, so the Board shows real pictures. */
export function stressProjects(projects: ProjectManifest[]): ProjectManifest[] {
  return projects.map(p => {
    if (p.id !== 'frostveil') return p;
    const own = p.assets ?? [];
    const extra = copies(own, STRESS.assets - own.length, (a, n) => ({
      ...a,
      id: `${a.id}-s${n}`,
      name: `${a.name} ${n}`,
    }));
    return { ...p, assets: [...own, ...extra] };
  });
}

/** The inbox at 200 items, copies of the fixture ones. */
export function stressInbox(inbox: InboxItem[]): InboxItem[] {
  return copies(inbox, STRESS.inbox, (item, n) =>
    n <= inbox.length ? item : { ...item, id: `${item.id}-s${n}`, name: `${item.name} ${n}` }
  );
}

/** Copies `count` lines of `log`, the step names kept and the text numbered. */
function longLog(log: LogEntry[]): LogEntry[] {
  return copies(log, STRESS.logLines, (line, n) =>
    n <= log.length ? line : { ...line, text: `${line.text} (${n})` }
  );
}

/** One response rewritten by the stress scenario. */
function wrap<T>(all: Record<string, (args: Args) => unknown>, cmd: string, grow: (found: T) => T) {
  const handler = all[cmd];
  if (!handler) return;
  all[cmd] = async args => grow((await handler(args)) as T);
}

/** Makes the library, clips, Lab, run log, ledger and MCP activity long. */
export function wrapStress(all: Record<string, (args: Args) => unknown>) {
  wrap<LibraryView>(all, 'library_get', view => ({
    ...view,
    items: copies(view.items, STRESS.library, (item, n) => ({
      ...item,
      entry: { ...item.entry, id: `${item.entry.id}-s${n}`, name: `${item.entry.name} ${n}` },
    })),
  }));
  wrap<AnimationsView>(all, 'animations_get', view => ({
    ...view,
    items: copies(view.items, STRESS.clips, (item, n) => ({
      ...item,
      clip: { ...item.clip, id: `${item.clip.id}-s${n}`, name: `${item.clip.name} ${n}` },
    })),
  }));
  wrap<LabView>(all, 'lab_get', view => ({
    ...view,
    directions: copies(view.directions, STRESS.directions, (d, n) =>
      n <= view.directions.length ? d : { ...d, id: `${d.id}-s${n}`, name: `${d.name} ${n}` }
    ),
  }));
  wrap<PipelineView>(all, 'pipeline_get', view => ({ ...view, log: longLog(view.log) }));
  wrap<LogEntry[]>(all, 'pipeline_log', longLog);
  wrap<Ledger>(all, 'credits_ledger', ledger => ({
    ...ledger,
    spends: copies(ledger.spends, STRESS.spends, (s, n) => ({
      ...s,
      job: `${s.job}-s${n}`,
      sentAtMs: s.sentAtMs === null ? null : s.sentAtMs - n * 3_600_000,
    })),
  }));
  wrap<McpStatus>(all, 'mcp_status', status => ({
    ...status,
    calls: copies(status.calls, STRESS.calls, (c, n) => ({ ...c, id: c.id * 1000 + n })),
  }));
}
