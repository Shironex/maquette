import { useEffect } from 'react';
import { Logo, NavRail, StatusBar, TrafficLights, cn, type StatusBarItem } from '@maquette/ui';
import { events } from '@/ipc/generated/bindings';
import { CHROME } from '@/ipc/runtime';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { useAppStore } from '@/state/app-store';
import { useInboxStore } from '@/state/inbox-store';
import { useLabStore } from '@/state/lab-store';
import { runStatusLine, runSummary } from '@/state/pipeline';
import { usePipelineStore } from '@/state/pipeline-store';
import { BoardView } from '@/features/board/BoardView';
import { InboxView } from '@/features/inbox/InboxView';
import { LabView } from '@/features/lab/LabView';
import { PipelineView } from '@/features/pipeline/PipelineView';
import { SettingsView } from '@/features/settings/SettingsView';
import { PlaceholderView } from '@/features/tools/PlaceholderView';
import { projectMeta, projectThumb } from './projectArt';
import { SETTINGS_VIEW, TOOL_VIEWS, type ViewId } from './views';

export function AppShell() {
  const view = useAppStore(s => s.view);
  const setView = useAppStore(s => s.setView);
  const workspace = useAppStore(s => s.workspace);
  const project = workspace?.project ?? null;
  const projectId = project?.id ?? null;
  const inboxCount = useInboxStore(s => s.snapshot?.items.length ?? 0);
  const running = usePipelineStore(s => Boolean(s.view?.active));
  const generating = useLabStore(s =>
    (s.view?.jobs ?? []).some(j => j.state === 'running' || j.state === 'queued')
  );
  useLiveData(projectId, workspace?.root ?? null);

  const items = TOOL_VIEWS.map(v =>
    v.id === 'inbox' && inboxCount > 0
      ? { ...v, badge: inboxCount }
      : (v.id === 'pipeline' && running) || (v.id === 'lab' && generating)
        ? { ...v, live: true }
        : v
  );

  return (
    <div className="flex h-full min-h-0 bg-app text-fg">
      <NavRail<ViewId>
        items={items}
        footer={SETTINGS_VIEW}
        value={view}
        onChange={setView}
        header={
          <div data-tauri-drag-region>
            <TrafficLights mode={CHROME} />
            <div
              data-tauri-drag-region
              className={cn(
                'flex items-center px-1.5 pb-3.5',
                CHROME === 'native' ? 'pt-1' : 'pt-[18px]'
              )}
            >
              <Logo size={24} />
            </div>
          </div>
        }
        project={
          project
            ? {
                name: project.name,
                meta: projectMeta(project),
                thumb: projectThumb(project),
                onClick: () => setView('settings'),
              }
            : undefined
        }
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex min-h-0 flex-1 flex-col">
          <CurrentView view={view} />
        </div>
        <ShellStatusBar />
      </div>
    </div>
  );
}

/**
 * Loads the pipeline and the inbox for the open project and keeps them
 * fresh from the runner and the inbox watcher.
 */
function useLiveData(projectId: string | null, root: string | null) {
  const loadPipeline = usePipelineStore(s => s.load);
  const onEvent = usePipelineStore(s => s.onEvent);
  const loadInbox = useInboxStore(s => s.load);
  const loadLab = useLabStore(s => s.load);
  const onConcept = useLabStore(s => s.onEvent);
  const onThumbnail = useInboxStore(s => s.onThumbnail);
  const setProject = useAppStore(s => s.setProject);
  useEffect(() => {
    if (projectId) void loadPipeline(projectId);
  }, [projectId, loadPipeline]);
  useEffect(() => {
    if (projectId) void loadLab(projectId);
  }, [projectId, loadLab]);
  useEffect(() => {
    if (root) void loadInbox();
  }, [root, loadInbox]);
  useEffect(() => {
    const stops = [
      events.pipelineUpdate.listen(e => onEvent(e.payload)),
      events.inboxChanged.listen(() => void loadInbox()),
      events.conceptUpdate.listen(e => onConcept(e.payload)),
      events.thumbnailReady.listen(e => {
        onThumbnail(e.payload);
        if (e.payload.project) setProject(e.payload.project);
      }),
    ];
    return () => {
      for (const stop of stops) void stop.then(fn => fn());
    };
  }, [onEvent, loadInbox, onThumbnail, setProject, onConcept]);
}

function CurrentView({ view }: { view: ViewId }) {
  switch (view) {
    case 'board':
      return <BoardView />;
    case 'pipeline':
      return <PipelineView />;
    case 'inbox':
      return <InboxView />;
    case 'lab':
      return <LabView />;
    case 'settings':
      return <SettingsView />;
    default:
      return <PlaceholderView view={view} />;
  }
}

function ShellStatusBar() {
  const workspace = useAppStore(s => s.workspace);
  const { reports, checking } = useAppStore(s => s.tools);
  const blender = reports.find(r => r.id === 'blender');
  const codex = reports.find(r => r.id === 'codex');
  const view = useAppStore(s => s.view);
  const labJobs = useLabStore(s => s.view?.jobs) ?? [];
  const generating = labJobs.filter(j => j.state === 'running').length;
  const waiting = labJobs.filter(j => j.state === 'queued').length;
  const codexItems: StatusBarItem[] =
    view === 'lab' && codex
      ? [
          codex.status === 'found'
            ? { label: 'Codex CLI, signed in', icon: 'terminal-window' }
            : { label: 'Codex CLI not ready', dot: 'offline' },
        ]
      : [];
  const labItems: StatusBarItem[] =
    generating || waiting
      ? [
          {
            label: generating
              ? `${generating} ${generating === 1 ? 'image' : 'images'} generating${waiting ? `, ${waiting} waiting` : ''}`
              : `${waiting} ${waiting === 1 ? 'image' : 'images'} waiting`,
            dot: 'running',
          },
        ]
      : [];
  const blenderItem: StatusBarItem = checking
    ? { label: 'Checking tools', dot: 'running' }
    : blender?.status === 'found'
      ? { label: `Blender ${blender.version ?? ''}, headless`.replace('  ', ' '), icon: 'cube' }
      : { label: 'Blender not found', dot: 'offline' };
  const inbox = useInboxStore(s => s.snapshot);
  const pipeline = usePipelineStore(s => s.view);
  const now = useNow(15_000);
  const latest = pipeline?.runs[0];
  const inboxDir =
    inbox?.dir ?? (workspace ? `${workspace.display}/${workspace.config.layout.inbox}` : null);
  const inboxItem: StatusBarItem[] = inboxDir
    ? [
        {
          label: `Inbox at ${inboxDir}${inbox?.watching ? ', watching' : ''}`,
          ...(inbox?.watching ? { dot: 'ok' as const } : { icon: 'tray' as const }),
        },
      ]
    : [];
  const runItems: StatusBarItem[] =
    latest && latest.state === 'running'
      ? [
          { label: runStatusLine(latest), dot: 'running' },
          { label: runSummary(latest), right: true },
        ]
      : latest
        ? [
            ...(latest.state === 'failed'
              ? [{ label: runStatusLine(latest), dot: 'offline' as const }]
              : []),
            {
              label: `Last run ${formatAgo(latest.finishedAtMs ?? latest.queuedAtMs ?? now, now)}`,
              right: true,
            },
          ]
        : [{ label: 'No runs yet', right: true }];
  const items: StatusBarItem[] = [
    { label: 'Local worker connected', dot: 'ok' },
    blenderItem,
    ...codexItems,
    ...(view === 'lab' ? [] : inboxItem),
    ...labItems,
    ...runItems,
  ];
  return <StatusBar items={items} />;
}
