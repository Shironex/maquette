/**
 * App wide state: what phase the window is in, the open workspace, the view
 * and the latest toolchain check. Feature state stays in its feature.
 */
import { create } from 'zustand';
import {
  commands,
  errorMessage,
  type Bootstrap,
  type ProjectManifest,
  type ToolId,
  type ToolReport,
  type WorkspaceConfig,
  type WorkspaceInUse,
  type WorkspaceSnapshot,
} from '@/ipc/client';
import { openAfterAsking } from './use-workspace-lock';
import { hashForView, viewFromHash, type ViewId } from '@/state/views';
import type { ToastStatus } from '@maquette/ui';
import { useSettingsGuard } from './use-settings-guard';
import { clock } from '@/lib/clock';

export type Phase = 'booting' | 'onboarding' | 'app' | 'failed';

export interface Notice {
  id: number;
  status: ToastStatus;
  title: string;
  detail?: string;
  /** One button on the notice, which also dismisses it. */
  action?: { label: string; run: () => void };
}

export interface ToolsState {
  reports: ToolReport[];
  checking: boolean;
  /** The tool a Locate or Reset is running for. */
  busyTool: ToolId | null;
  checkedAt: number | null;
}

interface AppStore {
  phase: Phase;
  boot: Bootstrap | null;
  bootError: string | null;
  workspace: WorkspaceSnapshot | null;
  view: ViewId;
  tools: ToolsState;
  notice: Notice | null;

  init: () => Promise<void>;
  /** Asks about the last workspace another computer has open, and opens it on a yes. */
  reopenInUse: (inUse: WorkspaceInUse) => Promise<void>;
  enterApp: (snapshot: WorkspaceSnapshot) => void;
  setWorkspace: (snapshot: WorkspaceSnapshot) => void;
  setConfig: (config: WorkspaceConfig) => void;
  /** Swaps in a changed project (new assets, a move) everywhere it shows. */
  setProject: (project: ProjectManifest) => void;
  addProject: (project: ProjectManifest) => void;
  /**
   * Opens a view. Leaving Settings with an invalid draft asks first, unless
   * `force` (a new workspace or project is not a way to leave).
   */
  setView: (view: ViewId, force?: boolean) => void;
  syncViewFromHash: () => void;
  detectTools: () => Promise<void>;
  locateTool: (tool: ToolId) => Promise<void>;
  notify: (notice: Omit<Notice, 'id'>) => void;
  dismissNotice: () => void;
}

let noticeId = 0;

function currentHash(): string {
  return typeof window === 'undefined' ? '' : window.location.hash;
}

/** The bootstrap on its way, if any. */
let booting: Promise<void> | null = null;

export const useAppStore = create<AppStore>((set, get) => ({
  phase: 'booting',
  boot: null,
  bootError: null,
  workspace: null,
  view: viewFromHash(currentHash()),
  tools: { reports: [], checking: false, busyTool: null, checkedAt: null },
  notice: null,

  init: () => {
    // StrictMode runs the effect twice: both share one bootstrap.
    booting ??= (async () => {
      try {
        const boot = await commands.appBootstrap();
        set({
          boot,
          workspace: boot.workspace,
          phase: boot.workspace ? 'app' : 'onboarding',
        });
        if (boot.lastWorkspaceError) {
          get().notify({
            status: 'failed',
            title: 'Could not reopen the last workspace',
            detail: boot.lastWorkspaceError,
          });
        }
        if (boot.workspaceInUse) void get().reopenInUse(boot.workspaceInUse);
      } catch (error) {
        set({ phase: 'failed', bootError: errorMessage(error) });
      }
    })().finally(() => {
      booting = null;
    });
    return booting;
  },

  reopenInUse: async inUse => {
    try {
      const snapshot = await openAfterAsking({ root: inUse.root, message: inUse.message });
      if (snapshot) get().enterApp(snapshot);
    } catch (error) {
      get().notify({
        status: 'failed',
        title: 'Could not reopen the last workspace',
        detail: errorMessage(error),
      });
    }
  },

  enterApp: snapshot => {
    set({ workspace: snapshot, phase: 'app' });
    get().setView('board', true);
  },

  setWorkspace: snapshot => set({ workspace: snapshot }),

  setConfig: config => {
    const workspace = get().workspace;
    if (workspace) set({ workspace: { ...workspace, config } });
  },

  setProject: project => {
    const workspace = get().workspace;
    if (!workspace) return;
    set({
      workspace: {
        ...workspace,
        project: workspace.project?.id === project.id ? project : workspace.project,
        projects: workspace.projects.map(p => (p.id === project.id ? project : p)),
      },
    });
  },

  addProject: project => {
    const workspace = get().workspace;
    if (!workspace || workspace.projects.some(p => p.id === project.id)) return;
    set({
      workspace: {
        ...workspace,
        // It opens only when none is open.
        project: workspace.project ?? project,
        projects: [...workspace.projects, project],
      },
    });
  },

  setView: (view, force) => {
    if (!force && get().view === 'settings' && view !== 'settings') {
      const guard = useSettingsGuard.getState();
      if (guard.problems.length > 0) {
        guard.askLeave(view);
        view = 'settings';
      }
    }
    set({ view });
    const hash = hashForView(view);
    if (typeof window !== 'undefined' && window.location.hash !== hash) {
      window.history.replaceState(null, '', hash);
    }
  },

  syncViewFromHash: () => get().setView(viewFromHash(currentHash())),

  detectTools: async () => {
    if (get().tools.checking) return;
    set(s => ({ tools: { ...s.tools, checking: true } }));
    try {
      const reports = await commands.toolchainDetect();
      set(s => ({ tools: { ...s.tools, reports, checking: false, checkedAt: clock.nowMs() } }));
    } catch (error) {
      set(s => ({ tools: { ...s.tools, checking: false } }));
      get().notify({
        status: 'failed',
        title: 'The tool check failed',
        detail: errorMessage(error),
      });
    }
  },

  locateTool: async tool => {
    const label = get().tools.reports.find(r => r.id === tool)?.label ?? tool;
    try {
      const path = await commands.dialogPickFile(`Locate ${label}`);
      if (!path) return;
      set(s => ({ tools: { ...s.tools, busyTool: tool } }));
      const report = await commands.toolchainLocate(tool, path);
      set(s => ({
        tools: {
          ...s.tools,
          busyTool: null,
          reports: s.tools.reports.map(r => (r.id === tool ? report : r)),
        },
      }));
      get().notify({
        status: 'done',
        title: `${label} located`,
        detail: report.version ?? undefined,
      });
    } catch (error) {
      set(s => ({ tools: { ...s.tools, busyTool: null } }));
      get().notify({
        status: 'failed',
        title: `Could not use that file for ${label}`,
        detail: errorMessage(error),
      });
    }
  },

  notify: notice => {
    noticeId += 1;
    set({ notice: { ...notice, id: noticeId } });
  },

  dismissNotice: () => set({ notice: null }),
}));
