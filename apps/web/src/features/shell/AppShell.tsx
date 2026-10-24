import { useEffect, useRef, useState } from 'react';
import { Logo, NavRail, StatusBar, TrafficLights, cn, type StatusBarItem } from '@maquette/ui';
import { events } from '@/ipc/generated/bindings';
import { CHROME, PLATFORM } from '@/ipc/runtime';
import { readFlag, writeFlag } from '@/lib/storage';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { mcpStatusItem } from '@/state/agents';
import { useAgentsStore } from '@/state/agents-store';
import { useAppStore } from '@/state/app-store';
import { useInboxStore } from '@/state/inbox-store';
import { useLabStore } from '@/state/lab-store';
import { runStatusLine, runSummary } from '@/state/pipeline';
import { usePipelineStore } from '@/state/pipeline-store';
import { useProvidersStore } from '@/state/providers-store';
import { isSwitcherShortcut, switcherShortcut } from '@/state/projects';
import { useProjectsStore } from '@/state/projects-store';
import { NewProjectDialog } from '@/features/project/NewProjectDialog';
import { ApprovalPrompt } from '@/features/agents/ApprovalPrompt';
import { BoardView } from '@/features/board/BoardView';
import { InboxView } from '@/features/inbox/InboxView';
import { LabView } from '@/features/lab/LabView';
import { InspectorView } from '@/features/inspector/InspectorView';
import { PipelineView } from '@/features/pipeline/PipelineView';
import { BatchSendPanel } from '@/features/providers/BatchSendPanel';
import { DownloadsMatches } from '@/features/providers/DownloadsMatches';
import { HandoffChecklist } from '@/features/providers/HandoffChecklist';
import { SendPanel } from '@/features/providers/SendPanel';
import { ObjectRefsDialog } from '@/features/lab/ObjectRefsDialog';
import { SettingsView } from '@/features/settings/SettingsView';
import { PlaceholderView } from '@/features/tools/PlaceholderView';
import { projectMeta, projectThumb } from './projectArt';
import { ProjectSwitcherMenu } from './ProjectSwitcherMenu';
import { SETTINGS_VIEW, TOOL_VIEWS, type ViewId } from './views';

/** Remembers a collapsed rail on this machine. */
const RAIL_COLLAPSED = 'maquette.rail.collapsed';

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
  const switcher = useRef<HTMLButtonElement>(null);
  const menuOpen = useProjectsStore(s => s.menuOpen);
  const toggleMenu = useProjectsStore(s => s.toggleMenu);
  const summary = useProjectsStore(s => s.list.find(p => p.id === projectId));
  const [collapsed, setCollapsed] = useState(() => readFlag(RAIL_COLLAPSED));
  const mac = PLATFORM === 'mac';
  const shortcut = switcherShortcut(mac);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!isSwitcherShortcut(e, mac)) return;
      // Also keeps the web view from opening its print dialog.
      e.preventDefault();
      toggleMenu();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [mac, toggleMenu]);
  const toggleRail = () =>
    setCollapsed(c => {
      writeFlag(RAIL_COLLAPSED, !c);
      return !c;
    });

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
        collapsed={collapsed}
        onToggleCollapsed={toggleRail}
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
                ref: switcher,
                name: project.name,
                meta: projectMeta(project),
                thumb: projectThumb(project, summary),
                expanded: menuOpen,
                shortcut,
                onClick: toggleMenu,
              }
            : undefined
        }
        projectMenu={
          <ProjectSwitcherMenu anchorRef={switcher} collapsed={collapsed} shortcut={shortcut} />
        }
      />
      <div className="flex min-w-0 flex-1 flex-col">
        <div className="flex min-h-0 flex-1 flex-col">
          <CurrentView view={view} />
        </div>
        <ShellStatusBar />
      </div>
      <HandoffChecklist />
      <BatchSendPanel />
      <SendPanel />
      <ObjectRefsDialog />
      <DownloadsMatches />
      <NewProjectDialog onClosed={() => switcher.current?.focus()} />
      <ApprovalPrompt />
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
  const loadAgents = useAgentsStore(s => s.load);
  const onAgents = useAgentsStore(s => s.onUpdate);
  const loadProviders = useProvidersStore(s => s.load);
  const onProvider = useProvidersStore(s => s.onEvent);
  const loadKeys = useProvidersStore(s => s.loadKeys);
  const loadProjects = useProjectsStore(s => s.load);
  useEffect(() => {
    // The rail shows the open project's own picture from the list.
    if (root) void loadProjects();
  }, [root, projectId, loadProjects]);
  useEffect(() => {
    if (projectId) void loadProviders(projectId);
  }, [projectId, loadProviders]);
  useEffect(() => {
    if (root) void loadKeys();
  }, [root, loadKeys]);
  useEffect(() => {
    if (root) void loadAgents();
  }, [root, loadAgents]);
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
      events.projectChanged.listen(e => setProject(e.payload.project)),
      events.mcpUpdate.listen(e => onAgents(e.payload)),
      events.providerUpdate.listen(e => onProvider(e.payload)),
    ];
    return () => {
      for (const stop of stops) void stop.then(fn => fn());
    };
  }, [onEvent, loadInbox, onThumbnail, setProject, onConcept, onAgents, onProvider]);
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
    case 'inspector':
      return <InspectorView />;
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
  const mcp = mcpStatusItem(useAgentsStore(s => s.status));
  const sends = useProvidersStore(s => s.jobs);
  const sending = sends.filter(j => j.state === 'running' || j.state === 'queued').length;
  const handWaiting = sends.filter(j => j.state === 'waiting').length;
  const sendParts = [
    sending ? `${sending} ${sending === 1 ? 'model' : 'models'} on the way` : null,
    handWaiting ? `${handWaiting} by hand` : null,
  ].filter(Boolean);
  const sendItems: StatusBarItem[] = sendParts.length
    ? [
        sending
          ? { label: sendParts.join(', '), dot: 'running' }
          : { label: `${handWaiting} waiting for your model`, icon: 'hand-pointing' },
      ]
    : [];
  const items: StatusBarItem[] = [
    { label: 'Local worker connected', dot: 'ok' },
    blenderItem,
    ...(mcp ? [mcp] : []),
    ...codexItems,
    ...(view === 'lab' ? [] : inboxItem),
    ...labItems,
    ...sendItems,
    ...runItems,
  ];
  return <StatusBar items={items} />;
}
