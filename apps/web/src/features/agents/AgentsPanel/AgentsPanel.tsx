import type { WorkspaceConfig } from '@/ipc/client';
import { AgentRuleRows } from '@/features/config/AgentRuleRows';
import { McpControls } from '@/features/config/McpControls';
import type { ConfigErrors } from '@/state/settings-form';
import { ApprovalCard, useApprovalStack } from '../ApprovalCard';
import { useAgentsPanel } from './AgentsPanel.hooks';
import { ActivityList } from './ActivityList';
import { ServerLine } from './ServerLine';
import { TokenRow } from './TokenRow';

export interface AgentsPanelProps {
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
  errors: ConfigErrors;
}

/** Settings > Agents: the server, its token, the rules, approvals and activity. */
export default function AgentsPanel({ config, onChange, errors }: AgentsPanelProps) {
  const { status } = useAgentsPanel();
  const stack = useApprovalStack(status?.approvals);
  const approvals = status?.approvals.map(a => (
    <ApprovalCard
      key={a.id}
      approval={a}
      collapsed={stack.collapsed(a.id)}
      onToggle={() => stack.toggle(a.id)}
      armed={stack.armed}
    />
  ));
  return (
    <>
      <ServerLine status={status} enabled={config.agents.mcpEnabled} />
      <McpControls config={config} onChange={onChange} errors={errors} density="compact" />
      <TokenRow />
      <AgentRuleRows config={config} onChange={onChange} density="compact" />
      {status && status.approvals.length > 0 ? (
        <div className="flex flex-col gap-2">
          <h3 className="text-meta font-semibold tracking-[0.02em] text-fg-mute uppercase">
            Waiting for you
          </h3>
          <div ref={stack.column} className="flex flex-col gap-2">
            {approvals}
          </div>
        </div>
      ) : null}
      <ActivityList status={status} />
    </>
  );
}
