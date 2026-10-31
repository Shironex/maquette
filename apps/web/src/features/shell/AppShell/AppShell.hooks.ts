import { useEffect, useRef, useState } from 'react';
import { events, type ImageSettings } from '@/ipc/generated/bindings';
import { PLATFORM } from '@/ipc/runtime';
import { readFlag, writeFlag } from '@/lib/storage';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { useAvatarStore } from '@/state/use-avatar-store';
import { useBuiltStore } from '@/state/use-built-store';
import { useInboxStore } from '@/state/use-inbox-store';
import { useAnimationsStore } from '@/state/use-animations-store';
import { useCodexUsageStore } from '@/state/use-codex-usage-store';
import { useImageApiStore } from '@/state/use-image-api-store';
import { useLibraryStore } from '@/state/use-library-store';
import { useLabStore } from '@/state/use-lab-store';
import { useLayoutStore } from '@/state/use-layout-store';
import { useMaterialsStore } from '@/state/use-materials-store';
import { useMotionStore } from '@/state/use-motion-store';
import { usePackStore } from '@/state/use-pack-store';
import { useTimelineStore } from '@/state/use-timeline-store';
import { usePathsStore } from '@/state/use-paths-store';
import { usePageStore } from '@/state/use-page-store';
import { usePrepareStore } from '@/state/use-prepare-store';
import { usePipelineStore } from '@/state/use-pipeline-store';
import { useGameStore } from '@/state/use-game-store';
import { useDownloadsStore } from '@/state/use-downloads-store';
import { useProvidersStore } from '@/state/use-providers-store';
import { useRefsStore } from '@/state/use-refs-store';
import { isSwitcherShortcut, switcherShortcut } from '@/state/projects';
import { useProjectsStore } from '@/state/use-projects-store';
import { LIBRARY_VIEWS, plannedViewsFor, toolViewsFor } from '@/state/views';
import { projectMeta, projectThumb } from '@/features/common/projectArt';
import { useQuitGuard } from '../QuitDialog';

/** Remembers a collapsed rail on this machine. */
const RAIL_COLLAPSED = 'maquette.rail.collapsed';

/** The rail's collapsed state, kept on this machine. */
export function useRailCollapsed() {
  const [collapsed, setCollapsed] = useState(() => readFlag(RAIL_COLLAPSED));
  const toggleRail = () =>
    setCollapsed(c => {
      writeFlag(RAIL_COLLAPSED, !c);
      return !c;
    });
  return { collapsed, toggleRail };
}

/** Opens and closes the project switcher on Cmd+P (macOS) or Ctrl+P. */
export function useSwitcherShortcut(mac: boolean, toggleMenu: () => void) {
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
}

/** The shell's view, rail items and project switcher, with live data for the open project. */
export function useAppShell() {
  const view = useAppStore(s => s.view);
  const setView = useAppStore(s => s.setView);
  const workspace = useAppStore(s => s.workspace);
  const project = workspace?.project ?? null;
  const projectId = project?.id ?? null;
  const inboxCount = useInboxStore(s => s.snapshot?.items.length ?? 0);
  const models = useLibraryStore(s => s.view?.items.length);
  const clips = useAnimationsStore(s => s.view?.items.length);
  const running = usePipelineStore(s => Boolean(s.view?.active));
  const generating = useLabStore(s =>
    (s.view?.jobs ?? []).some(j => j.state === 'running' || j.state === 'queued')
  );
  useLiveData(projectId, workspace?.root ?? null);
  useQuitGuard();
  const switcher = useRef<HTMLButtonElement>(null);
  const menuOpen = useProjectsStore(s => s.menuOpen);
  const toggleMenu = useProjectsStore(s => s.toggleMenu);
  const summary = useProjectsStore(s => s.list.find(p => p.id === projectId));
  const { collapsed, toggleRail } = useRailCollapsed();
  const mac = PLATFORM === 'mac';
  const shortcut = switcherShortcut(mac);
  useSwitcherShortcut(mac, toggleMenu);

  const items = toolViewsFor(project?.target).map(v =>
    v.id === 'inbox' && inboxCount > 0
      ? { ...v, badge: inboxCount }
      : (v.id === 'pipeline' && running) || (v.id === 'lab' && generating)
        ? { ...v, live: true }
        : v
  );
  const libraries = LIBRARY_VIEWS.map(v => ({
    ...v,
    count: v.id === 'library' ? models : clips,
  }));
  const switcherButton = project
    ? {
        ref: switcher,
        name: project.name,
        meta: projectMeta(project),
        thumb: projectThumb(project, summary),
        expanded: menuOpen,
        shortcut,
        onClick: toggleMenu,
      }
    : undefined;

  return {
    view,
    setView,
    items,
    groups: [{ title: 'Libraries', items: libraries }],
    planned: plannedViewsFor(project?.target),
    target: project?.target,
    collapsed,
    toggleRail,
    switcher,
    switcherButton,
    shortcut,
    focusSwitcher: () => switcher.current?.focus(),
  };
}

/**
 * Loads the pipeline, the inbox, the material tweaks and the built assets
 * for the open project and keeps them fresh from the runner, the inbox
 * watcher, builds and agents.
 */
/** An agent changed the Lab's image backends: the Lab and Settings follow. */
function onImageSettings(images: ImageSettings) {
  const { workspace, setConfig } = useAppStore.getState();
  if (workspace) setConfig({ ...workspace.config, lab: { ...workspace.config.lab, images } });
  void useImageApiStore.getState().load();
}

export function useLiveData(projectId: string | null, root: string | null) {
  const loadPipeline = usePipelineStore(s => s.load);
  const onEvent = usePipelineStore(s => s.onEvent);
  const refreshPipeline = usePipelineStore(s => s.refresh);
  const loadInbox = useInboxStore(s => s.load);
  const loadLab = useLabStore(s => s.load);
  const onConcept = useLabStore(s => s.onEvent);
  const onThumbnail = useInboxStore(s => s.onThumbnail);
  const onPreview = useGameStore(s => s.onThumbnail);
  const setProject = useAppStore(s => s.setProject);
  const loadAgents = useAgentsStore(s => s.load);
  const onAgents = useAgentsStore(s => s.onUpdate);
  const loadProviders = useProvidersStore(s => s.load);
  const onProvider = useProvidersStore(s => s.onEvent);
  const loadKeys = useProvidersStore(s => s.loadKeys);
  const loadProjects = useProjectsStore(s => s.load);
  const onLayoutChanged = useLayoutStore(s => s.onChanged);
  const loadMaterials = useMaterialsStore(s => s.load);
  const setMaterialsRoot = useMaterialsStore(s => s.setRoot);
  const setPrepareRoot = usePrepareStore(s => s.setRoot);
  const reloadPrepare = usePrepareStore(s => s.reload);
  const loadDownloads = useDownloadsStore(s => s.load);
  const onArrival = useDownloadsStore(s => s.onArrival);
  const onDownloadThumb = useDownloadsStore(s => s.onThumbnail);
  const loadBuilt = useBuiltStore(s => s.load);
  const onBuilt = useBuiltStore(s => s.onChanged);
  const onPageServer = usePageStore(s => s.onServer);
  useEffect(() => {
    if (projectId) void loadBuilt(projectId);
  }, [root, projectId, loadBuilt]);
  useEffect(() => {
    // The watch is workspace wide, so a project switch needs no reload.
    if (root) void loadDownloads();
  }, [root, loadDownloads]);
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
    // Before the load: another workspace drops the tweaks of this one.
    setMaterialsRoot(root);
    setPrepareRoot(root);
    if (projectId) void loadMaterials(projectId);
  }, [root, projectId, setMaterialsRoot, setPrepareRoot, loadMaterials]);
  useEffect(() => {
    if (root) void loadInbox();
  }, [root, loadInbox]);
  useEffect(() => {
    // The Board and Inspector show updates waiting, so the library loads with the workspace.
    if (root) void useLibraryStore.getState().load();
    // The rail counts the clips.
    if (root) void useAnimationsStore.getState().load();
    // The status bar's Codex usage, as the shell last read it.
    if (root) void useCodexUsageStore.getState().load();
  }, [root]);
  useEffect(() => {
    const stops = [
      events.pipelineUpdate.listen(e => onEvent(e.payload)),
      events.pipelineChanged.listen(e => void refreshPipeline(e.payload.project)),
      events.layoutChanged.listen(e => {
        void onLayoutChanged(e.payload.project);
        void refreshPipeline(e.payload.project);
        void useMotionStore.getState().onChanged(e.payload.project);
        void useTimelineStore.getState().onChanged(e.payload.project);
        void usePathsStore.getState().onChanged(e.payload.project);
      }),
      events.inboxChanged.listen(() => void loadInbox()),
      events.conceptUpdate.listen(e => {
        onConcept(e.payload);
        useRefsStore.getState().onConcept(e.payload);
      }),
      events.thumbnailReady.listen(e => {
        onThumbnail(e.payload);
        onPreview(e.payload);
        onDownloadThumb(e.payload);
        if (e.payload.project) setProject(e.payload.project);
      }),
      events.projectChanged.listen(e => {
        setProject(e.payload.project);
        useRefsStore.getState().onProject(e.payload.project);
        useLibraryStore.getState().onChanged();
      }),
      events.libraryChanged.listen(() => {
        useLibraryStore.getState().onChanged();
        useAnimationsStore.getState().onChanged();
      }),
      events.packChanged.listen(e => usePackStore.getState().onChanged(e.payload.record)),
      events.mcpUpdate.listen(e => onAgents(e.payload)),
      events.providerUpdate.listen(e => onProvider(e.payload)),
      events.materialsChanged.listen(e => void loadMaterials(e.payload.project)),
      events.prepareChanged.listen(e => {
        void reloadPrepare(e.payload.project);
        void useMotionStore.getState().onChanged(e.payload.project);
        void useTimelineStore.getState().onChanged(e.payload.project);
      }),
      events.downloadsArrival.listen(e => onArrival(e.payload)),
      events.avatarChanged.listen(e =>
        useAvatarStore.getState().onChanged(e.payload.project, e.payload.asset)
      ),
      events.builtChanged.listen(e => onBuilt(e.payload.project)),
      events.pageServerChanged.listen(e => onPageServer(e.payload)),
      events.codexUsageUpdate.listen(e => useCodexUsageStore.getState().apply(e.payload)),
      events.imageSettingsChanged.listen(e => onImageSettings(e.payload)),
    ];
    return () => {
      for (const stop of stops) void stop.then(fn => fn());
    };
  }, [
    onEvent,
    refreshPipeline,
    loadInbox,
    onThumbnail,
    onPreview,
    onDownloadThumb,
    setProject,
    onConcept,
    onAgents,
    onProvider,
    onLayoutChanged,
    loadMaterials,
    reloadPrepare,
    onArrival,
    onBuilt,
    onPageServer,
  ]);
}
