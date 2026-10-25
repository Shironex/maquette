import { Icon, Toggle, TopBar, cn } from '@maquette/ui';
import type { WorkspaceConfig } from '@/ipc/client';
import { PANEL_TITLE, Panel } from '@/features/common/Panel';
import { AgentsPanel } from '@/features/agents/AgentsPanel';
import { BudgetFields } from '@/features/config/BudgetFields';
import { CompressionFields } from '@/features/config/CompressionFields';
import { KeyFields } from '@/features/providers/KeyFields';
import { ProviderPresetFields } from '@/features/providers/PresetFields';
import { RecheckButton, ToolList } from '@/features/toolchain/ToolList';
import { SAVE_STATUS_LABEL, patchSection, type SaveStatus } from '@/state/settings-form';
import { ACTIVE } from '../sections';
import { InvalidSettingsDialog } from '../InvalidSettingsDialog';
import { ProjectSection } from '../ProjectSection';
import { SectionNav } from '../SectionNav';
import { WorkspaceSection } from '../WorkspaceSection';
import { useSettingsScreen } from './SettingsScreen.hooks';

const STATUS_TONE: Record<SaveStatus, string> = {
  saved: 'text-fg-mute',
  pending: 'text-fg-mute',
  saving: 'text-fg-mute',
  invalid: 'text-danger',
  error: 'text-danger',
};

export default function SettingsScreen() {
  const { draft, setDraft, status, errors, problems, discardInvalid, leaveTo, active, go } =
    useSettingsScreen();
  if (!draft) return null;

  const props = {
    config: draft,
    onChange: (c: WorkspaceConfig) => setDraft(c),
    errors,
    density: 'compact' as const,
  };
  const statusIcon =
    status === 'saved'
      ? 'check'
      : status === 'saving' || status === 'pending'
        ? 'circle-notch'
        : 'warning-circle';

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
              name={statusIcon}
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
                id="settings-providers"
                title="3D providers"
                help="API keys, and the presets a Send to 3D starts from"
                className={cn('gap-3 px-4 py-3.5', active === 'providers' && ACTIVE)}
              >
                <KeyFields />
                <div className="h-px bg-line" />
                <ProviderPresetFields {...props} />
              </Panel>
              <Panel
                id="settings-budgets"
                title="Budgets and compression"
                help="What the scene may weigh, and the compression every model starts from"
                className={cn('gap-3 px-4 py-3.5', active === 'budgets' && ACTIVE)}
              >
                <div className="grid grid-cols-2 gap-2.5">
                  <BudgetFields {...props} />
                  <CompressionFields {...props} />
                </div>
                <p className="text-meta text-fg-mute">
                  A model can change its own compression in Pipeline &gt; Compression presets.
                </p>
              </Panel>
              <Panel
                id="settings-agents"
                title="Agents"
                help="Built-in MCP server so Claude Code and Codex can drive Maquette"
                actions={
                  <Toggle
                    aria-label="MCP server"
                    checked={draft.agents.mcpEnabled}
                    onChange={mcpEnabled => setDraft(patchSection(draft, 'agents', { mcpEnabled }))}
                  />
                }
                className={cn('gap-3 px-4 py-3.5', active === 'agents' && ACTIVE)}
              >
                <AgentsPanel config={draft} onChange={c => setDraft(c)} errors={errors} />
              </Panel>
            </div>
          </div>
        </div>
      </main>
      {leaveTo ? (
        <InvalidSettingsDialog
          leaveTo={leaveTo}
          problems={problems}
          discardInvalid={discardInvalid}
        />
      ) : null}
    </>
  );
}
