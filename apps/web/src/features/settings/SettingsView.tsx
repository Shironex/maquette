import { useEffect, useState, type ReactNode } from 'react';
import {
  Button,
  Icon,
  PathField,
  TextField,
  Toggle,
  TopBar,
  cn,
  type IconName,
} from '@maquette/ui';
import { commands, errorMessage, type WorkspaceConfig } from '@/ipc/client';
import { PANEL_TITLE, Panel } from '@/features/common/Panel';
import {
  AgentRuleRows,
  BudgetFields,
  CompressionFields,
  McpControls,
  TripoFields,
} from '@/features/config/ConfigFields';
import { projectThumb } from '@/features/shell/projectArt';
import { RecheckButton, ToolList } from '@/features/toolchain/ToolList';
import { formatBytes, homePath } from '@/lib/format';
import { useAppStore } from '@/state/app-store';
import { SAVE_STATUS_LABEL, patchSection, type SaveStatus } from '@/state/settings-form';
import { summarizeTools } from '@/state/toolchain';
import { useAutosave } from './useAutosave';

type SectionId = 'workspace' | 'toolchain' | 'tripo' | 'budgets' | 'agents' | 'project';

const SECTIONS: readonly { id: SectionId; label: string; icon: IconName }[] = [
  { id: 'workspace', label: 'Workspace', icon: 'folder' },
  { id: 'toolchain', label: 'Toolchain', icon: 'wrench' },
  { id: 'tripo', label: 'Tripo defaults', icon: 'cube' },
  { id: 'budgets', label: 'Budgets and compression', icon: 'gauge' },
  { id: 'agents', label: 'Agents', icon: 'robot' },
];

const STATUS_TONE: Record<SaveStatus, string> = {
  saved: 'text-fg-mute',
  pending: 'text-fg-mute',
  saving: 'text-fg-mute',
  invalid: 'text-danger',
  error: 'text-danger',
};

/** Remounts per workspace, so a switched workspace never shows the old draft. */
export function SettingsView() {
  const root = useAppStore(s => s.workspace?.root);
  return <SettingsScreen key={root} />;
}

function SettingsScreen() {
  const { draft, setDraft, status, errors } = useAutosave();
  const [active, setActive] = useState<SectionId>('workspace');
  if (!draft) return null;

  const props = {
    config: draft,
    onChange: (c: WorkspaceConfig) => setDraft(c),
    errors,
    density: 'compact' as const,
  };
  const go = (id: SectionId) => {
    setActive(id);
    document
      .getElementById(`settings-${id}`)
      ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  };

  return (
    <>
      <TopBar
        lead={<h1 className="text-panel-title">Settings</h1>}
        right={
          <span
            role="status"
            className={cn('flex items-center gap-1.5 text-meta', STATUS_TONE[status])}
          >
            <Icon
              name={
                status === 'saved'
                  ? 'check'
                  : status === 'saving' || status === 'pending'
                    ? 'circle-notch'
                    : 'warning-circle'
              }
              spin={status === 'saving'}
              size={14}
              className={status === 'saved' ? 'text-sage' : undefined}
            />
            {SAVE_STATUS_LABEL[status]}
          </span>
        }
      />
      <main className="flex min-h-0 flex-1">
        <SectionNav active={active} onSelect={go} />
        <div className="min-w-0 flex-1 overflow-y-auto px-6 py-[18px]">
          <div className="grid grid-cols-2 items-start gap-3.5">
            <div className="flex flex-col gap-3.5">
              <WorkspaceSection active={active === 'workspace'} config={draft} />
              <section
                id="settings-toolchain"
                aria-labelledby="settings-toolchain-title"
                className={cn(
                  'flex flex-col rounded-card border border-line bg-panel pt-3.5 pb-1',
                  active === 'toolchain' && ACTIVE
                )}
              >
                <div className="flex items-center px-4 pb-2">
                  <h2 id="settings-toolchain-title" className={cn(PANEL_TITLE, 'flex-1')}>
                    Toolchain
                  </h2>
                  <RecheckButton />
                </div>
                <ToolList density="compact" />
              </section>
              <ProjectSection active={active === 'project'} />
            </div>
            <div className="flex flex-col gap-3.5">
              <Panel
                id="settings-tripo"
                title="Tripo defaults"
                className={cn('gap-3 px-4 py-3.5', active === 'tripo' && ACTIVE)}
              >
                <TripoFields {...props} />
              </Panel>
              <Panel
                id="settings-budgets"
                title="Budgets and compression"
                className={cn('gap-3 px-4 py-3.5', active === 'budgets' && ACTIVE)}
              >
                <div className="grid grid-cols-2 gap-2.5">
                  <BudgetFields {...props} />
                  <CompressionFields {...props} />
                </div>
                <p className="text-meta text-fg-mute">
                  Per-model overrides will live in the Pipeline presets.
                </p>
              </Panel>
              <Panel
                id="settings-agents"
                title="Agents"
                help={
                  draft.agents.mcpEnabled
                    ? `Port ${draft.agents.mcpPort} reserved for Claude Code and Codex. The server starts in a later version.`
                    : 'Built-in MCP server so Claude Code and Codex can drive Maquette'
                }
                actions={
                  <Toggle
                    aria-label="MCP server"
                    checked={draft.agents.mcpEnabled}
                    onChange={mcpEnabled => setDraft(patchSection(draft, 'agents', { mcpEnabled }))}
                  />
                }
                className={cn('gap-3 px-4 py-3.5', active === 'agents' && ACTIVE)}
              >
                <McpControls {...props} />
                <AgentRuleRows {...props} />
              </Panel>
            </div>
          </div>
        </div>
      </main>
    </>
  );
}

const ACTIVE = 'shadow-[0_0_0_1px_var(--clay-line)]';

function SectionNav({
  active,
  onSelect,
}: {
  active: SectionId;
  onSelect: (id: SectionId) => void;
}) {
  const reports = useAppStore(s => s.tools.reports);
  const project = useAppStore(s => s.workspace?.project);
  const summary = summarizeTools(reports);
  const navItem = (id: SectionId, label: string, lead: ReactNode, meta?: ReactNode) => (
    <button
      key={id}
      type="button"
      aria-current={active === id ? 'true' : undefined}
      onClick={() => onSelect(id)}
      className={cn(
        'flex h-[34px] w-full cursor-pointer items-center gap-2.5 rounded-control px-2.5 text-left font-medium text-fg-dim hover:bg-hover hover:text-fg',
        active === id && 'bg-clay-soft text-fg hover:bg-clay-soft [&>svg]:text-clay-text'
      )}
    >
      {lead}
      <span className="min-w-0 flex-1 truncate">{label}</span>
      {meta}
    </button>
  );
  return (
    <nav
      aria-label="Settings sections"
      className="flex w-[232px] flex-none flex-col gap-0.5 border-r border-line px-3 py-[18px]"
    >
      {SECTIONS.map(s =>
        navItem(
          s.id,
          s.label,
          <Icon name={s.icon} className={active === s.id ? 'text-clay-text' : 'text-fg-mute'} />,
          s.id === 'toolchain' && reports.length ? (
            <span
              className={cn(
                'font-mono text-[11px]',
                summary.warnings || summary.blocked ? 'text-amber' : 'text-fg-mute'
              )}
            >
              {summary.found} of {summary.total}
            </span>
          ) : undefined
        )
      )}
      {project ? (
        <>
          <div className="px-2 pt-[22px] pb-1.5 text-caption tracking-[0.06em] text-fg-mute uppercase">
            This project
          </div>
          {navItem(
            'project',
            project.name,
            projectThumb(project) ? (
              <img
                src={projectThumb(project)}
                alt=""
                className="size-4 rounded-[4px] object-cover"
              />
            ) : (
              <Icon name="cube" className="text-fg-mute" />
            )
          )}
        </>
      ) : null}
    </nav>
  );
}

function WorkspaceSection({ active, config }: { active: boolean; config: WorkspaceConfig }) {
  const workspace = useAppStore(s => s.workspace);
  const setWorkspace = useAppStore(s => s.setWorkspace);
  const notify = useAppStore(s => s.notify);
  const [root, setRoot] = useState(workspace?.display ?? '');
  const [note, setNote] = useState<{ tone: 'found' | 'missing'; text: string } | undefined>();
  const [cache, setCache] = useState<number | null>(null);
  const [clearing, setClearing] = useState(false);

  useEffect(() => {
    let live = true;
    commands
      .workspaceCacheSize()
      .then(info => live && setCache(info.bytes ?? 0))
      .catch(() => live && setCache(null));
    return () => {
      live = false;
    };
  }, [workspace?.root]);

  const switchTo = async (path: string) => {
    if (!path.trim() || path === workspace?.display || path === workspace?.root) {
      setNote(undefined);
      return;
    }
    try {
      const found = await commands.workspaceInspect(path);
      if (found.state !== 'workspace') {
        setNote({
          tone: 'missing',
          text: 'Pick a folder that holds maquette.json. Moving a workspace arrives later.',
        });
        return;
      }
      const snapshot = await commands.workspaceOpen(found.path);
      setWorkspace(snapshot);
      setRoot(snapshot.display);
      setNote({ tone: 'found', text: 'Switched to this workspace.' });
    } catch (error) {
      setNote({ tone: 'missing', text: errorMessage(error) });
    }
  };

  const browse = async () => {
    const picked = await commands
      .dialogPickFolder('Open a Maquette workspace', root)
      .catch(() => null);
    if (picked) {
      setRoot(picked);
      await switchTo(picked);
    }
  };

  const clear = async () => {
    setClearing(true);
    try {
      const info = await commands.workspaceClearCache();
      setCache(info.bytes ?? 0);
      notify({
        status: 'done',
        title: 'Cache cleared',
        detail: 'Unchanged steps will run again on the next build.',
      });
    } catch (error) {
      notify({ status: 'failed', title: 'Could not clear the cache', detail: errorMessage(error) });
    } finally {
      setClearing(false);
    }
  };

  const { layout } = config;
  const folders = [
    [layout.projects, 'one folder per project'],
    [layout.inbox, 'Tripo models land here'],
    [layout.cache, 'bake and export cache, safe to clear'],
  ] as const;

  return (
    <Panel
      id="settings-workspace"
      title="Workspace"
      className={cn('gap-3 px-4 py-3.5', active && ACTIVE)}
    >
      <PathField
        label="Root folder"
        value={root}
        onChange={setRoot}
        onCommit={path => void switchTo(path)}
        onBrowse={() => void browse()}
        note={note}
      />
      <div className="flex flex-col gap-1.5 rounded-[8px] border border-line bg-app px-3 py-2.5">
        {folders.map(([name, text]) => (
          <div key={name} className="flex gap-3">
            <span className="w-[84px] font-mono text-[12px] tracking-[-0.01em]">{name}/</span>
            <span className="text-meta text-fg-mute">{text}</span>
          </div>
        ))}
      </div>
      <div className="flex items-center gap-3">
        <div className="flex-1">
          Cache
          <span className="ml-1.5 font-mono text-[12px] text-fg-mute">
            {cache === null ? 'unknown' : formatBytes(cache)}
          </span>
        </div>
        <Button size="sm" loading={clearing} disabled={!cache} onClick={() => void clear()}>
          Clear cache
        </Button>
      </div>
    </Panel>
  );
}

function ProjectSection({ active }: { active: boolean }) {
  const workspace = useAppStore(s => s.workspace);
  const setWorkspace = useAppStore(s => s.setWorkspace);
  const notify = useAppStore(s => s.notify);
  const home = useAppStore(s => s.boot?.home);
  const project = workspace?.project;
  const shownRepo = project?.siteRepo ? homePath(project.siteRepo, home) : '';
  const [name, setName] = useState(project?.name ?? '');
  const [repo, setRepo] = useState(shownRepo);

  useEffect(() => {
    setName(project?.name ?? '');
    setRepo(shownRepo);
  }, [project?.id, project?.name, shownRepo]);

  if (!workspace || !project) return null;

  const save = async (nextName: string, nextRepo: string) => {
    if (nextName.trim() === project.name && nextRepo.trim() === shownRepo) return;
    try {
      const updated = await commands.projectUpdate(
        project.id,
        nextName.trim(),
        nextRepo.trim() || null
      );
      setWorkspace({
        ...workspace,
        project: updated,
        projects: workspace.projects.map(p => (p.id === updated.id ? updated : p)),
      });
    } catch (error) {
      setName(project.name);
      notify({
        status: 'failed',
        title: 'Could not save the project',
        detail: errorMessage(error),
      });
    }
  };

  const browse = async () => {
    const picked = await commands.dialogPickFolder('Pick the site repo', repo).catch(() => null);
    if (picked) {
      setRepo(picked);
      await save(name, picked);
    }
  };

  return (
    <Panel
      id="settings-project"
      title="Project"
      help={`${project.name} only`}
      helpInline
      className={cn('gap-3 px-4 py-3.5 [&>div:first-child]:justify-between', active && ACTIVE)}
    >
      <div className="grid grid-cols-[minmax(0,0.7fr)_minmax(0,1.8fr)] gap-2.5">
        <TextField
          label="Name"
          value={name}
          onChange={e => setName(e.target.value)}
          onBlur={() => void save(name, repo)}
          onKeyDown={e => {
            if (e.key === 'Enter') e.currentTarget.blur();
          }}
        />
        <PathField
          label="Target site repo"
          value={repo}
          placeholder="Optional"
          onChange={setRepo}
          onCommit={value => void save(name, value)}
          onBrowse={() => void browse()}
          browseLabel="Browse for the site repo"
          compactBrowse
        />
      </div>
    </Panel>
  );
}
