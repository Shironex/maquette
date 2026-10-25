import { useState } from 'react';
import { Button, Icon, IconButton, cn } from '@maquette/ui';
import type { CallOutcome, McpCall, McpStatus, WorkspaceConfig } from '@/ipc/client';
import { AgentRuleRows, McpControls, copyText } from '@/features/config/ConfigFields';
import { useNow } from '@/lib/useNow';
import { useAgentsStore } from '@/state/use-agents-store';
import { useAppStore } from '@/state/use-app-store';
import { clockTime, serverLine } from '@/state/agents';
import type { ConfigErrors } from '@/state/settings-form';
import { maskToken } from '@/state/settings-form';
import { ApprovalCard } from './ApprovalPrompt';

interface Props {
  config: WorkspaceConfig;
  onChange: (config: WorkspaceConfig) => void;
  errors: ConfigErrors;
}

/** Settings > Agents: the server, its token, the rules, approvals and activity. */
export function AgentsPanel({ config, onChange, errors }: Props) {
  const status = useAgentsStore(s => s.status);
  return (
    <>
      <ServerLine status={status} enabled={config.agents.mcpEnabled} />
      <McpControls
        config={config}
        onChange={onChange}
        errors={errors}
        density="compact"
        token={status?.token ?? null}
      />
      {status?.token ? <TokenRow token={status.token} /> : null}
      <AgentRuleRows config={config} onChange={onChange} density="compact" />
      {status && status.approvals.length > 0 ? (
        <div className="flex flex-col gap-2">
          <h3 className="text-meta font-semibold tracking-[0.02em] text-fg-mute uppercase">
            Waiting for you
          </h3>
          {status.approvals.map(a => (
            <ApprovalCard key={a.id} approval={a} />
          ))}
        </div>
      ) : null}
      <ActivityList status={status} />
    </>
  );
}

function ServerLine({ status, enabled }: { status: McpStatus | null; enabled: boolean }) {
  const line = serverLine(status, enabled);
  return (
    <p
      role="status"
      className={cn(
        'flex items-center gap-1.5 text-meta',
        line.tone === 'ok' && 'text-sage',
        line.tone === 'error' && 'text-danger',
        line.tone === 'off' && 'text-fg-mute'
      )}
    >
      {line.tone === 'ok' ? (
        <span className="size-2 rounded-full bg-sage shadow-[0_0_0_3px_var(--color-sage-soft)]" />
      ) : (
        <Icon name={line.tone === 'error' ? 'warning-circle' : 'info'} size={14} />
      )}
      {line.text}
    </p>
  );
}

function TokenRow({ token }: { token: string }) {
  const [shown, setShown] = useState(false);
  const renew = useAgentsStore(s => s.renewToken);
  const notify = useAppStore(s => s.notify);
  return (
    <div className="flex items-center gap-2.5 rounded-control border border-line bg-app px-3 py-2">
      <Icon name="lock-simple" size={14} className="text-fg-mute" />
      <span className="text-meta text-fg-mute">Bearer token</span>
      <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg-dim">
        {shown ? token : maskToken(token)}
      </code>
      <IconButton
        icon="eye"
        label={shown ? 'Hide the token' : 'Show the token'}
        aria-pressed={shown}
        size="sm"
        variant="ghost"
        onClick={() => setShown(s => !s)}
      />
      <IconButton
        icon="copy"
        label="Copy the token"
        size="sm"
        variant="ghost"
        onClick={() =>
          void copyText(token).then(ok =>
            notify(
              ok
                ? { status: 'done', title: 'Token copied' }
                : { status: 'failed', title: 'Could not copy' }
            )
          )
        }
      />
      <Button
        size="sm"
        variant="ghost"
        title="Agents set up with the old token stop working"
        onClick={() => void renew()}
      >
        New token
      </Button>
    </div>
  );
}

const OUTCOME: Record<CallOutcome, { label: string; className: string }> = {
  ok: { label: 'ok', className: 'text-sage' },
  waiting: { label: 'waiting', className: 'text-amber' },
  error: { label: 'error', className: 'text-danger' },
  refused: { label: 'refused', className: 'text-danger' },
};

function ActivityList({ status }: { status: McpStatus | null }) {
  const clear = useAgentsStore(s => s.clearActivity);
  const calls = status?.calls ?? [];
  useNow(30_000);
  return (
    <div className="flex flex-col gap-1.5">
      <div className="flex items-center justify-between">
        <h3 className="text-meta font-semibold tracking-[0.02em] text-fg-mute uppercase">
          Activity
        </h3>
        {calls.length ? (
          <Button size="sm" variant="ghost" onClick={() => void clear()}>
            Clear
          </Button>
        ) : null}
      </div>
      {calls.length === 0 ? (
        <p className="rounded-control border border-dashed border-line px-3 py-3 text-meta text-fg-mute">
          No calls yet. Each tool call an agent makes shows up here.
        </p>
      ) : (
        <ol
          aria-label="MCP activity"
          className="flex max-h-[260px] list-none flex-col overflow-y-auto rounded-control border border-line bg-app"
        >
          {calls.map(call => (
            <ActivityRow key={call.id} call={call} />
          ))}
        </ol>
      )}
    </div>
  );
}

function ActivityRow({ call }: { call: McpCall }) {
  const outcome = OUTCOME[call.outcome];
  return (
    <li
      className="grid grid-cols-[58px_148px_minmax(0,1fr)_52px] items-baseline gap-2 border-t border-line px-3 py-[7px] first:border-t-0"
      title={call.message ?? undefined}
    >
      <span className="font-mono text-[11.5px] text-fg-mute tabular-nums">
        {clockTime(call.atMs ?? 0)}
      </span>
      <span className="truncate font-mono text-[12px] text-fg">{call.tool}</span>
      <span className="truncate text-meta text-fg-mute">
        {call.outcome === 'ok' || call.outcome === 'waiting'
          ? call.detail || ' '
          : (call.message ?? call.detail)}
      </span>
      <span className={cn('text-right text-meta font-medium', outcome.className)}>
        {outcome.label}
      </span>
    </li>
  );
}
