import { Button, Icon, cn } from '@maquette/ui';
import type { Approval } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { useNow } from '@/lib/useNow';
import { approvalTitle } from '@/state/agents';
import { useAgentsStore } from '@/state/agents-store';
import { useAppStore } from '@/state/app-store';

/**
 * What an agent asked that waits for a person: a run stopped before Ship
 * ("Ask before shipping"), or a send to Tripo or Meshy that would spend
 * credits. Shown over every view until someone approves or denies it.
 */
export function ApprovalPrompt() {
  const approvals = useAgentsStore(s => s.status?.approvals);
  const view = useAppStore(s => s.view);
  // Settings > Agents lists them in place.
  if (!approvals?.length || view === 'settings') return null;
  return (
    <div className="pointer-events-none fixed top-[68px] right-5 z-40 flex w-[400px] flex-col gap-2.5">
      {approvals.map(a => (
        <ApprovalCard key={a.id} approval={a} floating />
      ))}
    </div>
  );
}

export function ApprovalCard({ approval, floating }: { approval: Approval; floating?: boolean }) {
  const decide = useAgentsStore(s => s.decide);
  const deciding = useAgentsStore(s => s.deciding === approval.id);
  const projects = useAppStore(s => s.workspace?.projects);
  const site = projects?.find(p => p.id === approval.project);
  const now = useNow(15_000);
  const titleId = `${approval.id}-title`;
  const send = approval.kind === 'send-to-3d';
  return (
    <section
      role="alertdialog"
      aria-labelledby={titleId}
      className={cn(
        'pointer-events-auto flex flex-col gap-3 rounded-card border border-amber/40 bg-panel p-3.5',
        floating && 'shadow-window'
      )}
    >
      <div className="flex items-start gap-3">
        <span className="grid size-8 flex-none place-items-center rounded-[9px] bg-amber-soft text-amber">
          <Icon name={send ? 'cube' : 'rocket-launch'} size={17} />
        </span>
        <div className="min-w-0 flex-1">
          <h2 id={titleId} className="text-emphasis leading-5 font-semibold">
            {approvalTitle(approval, site?.name)}
          </h2>
          {send ? (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent asked over MCP. This spends {approval.service} credits; nothing is sent until
              you approve.
            </p>
          ) : (
            <p className="mt-0.5 text-meta text-fg-mute">
              An agent started this run over MCP. Everything before {approval.step} is done;{' '}
              {approval.step} writes to {site?.siteRepo ? 'the site repo' : 'the shipped folder'}.
            </p>
          )}
        </div>
        <span className="flex-none text-meta text-fg-mute tabular-nums">
          {formatAgo(approval.requestedAtMs ?? now, now)}
        </span>
      </div>
      {send && approval.items.length > 0 ? (
        <div className="ml-11 rounded-[7px] border border-line bg-raised text-meta">
          <ul className="m-0 flex max-h-[180px] list-none flex-col overflow-y-auto p-0">
            {approval.items.map(item => (
              <li
                key={item.job}
                className="flex items-baseline gap-2 border-t border-line px-2.5 py-1 first:border-t-0"
              >
                <span className="flex-none font-medium text-fg">{item.assetName}</span>
                <span className="min-w-0 truncate text-fg-mute" title={item.detail}>
                  {item.detail}
                </span>
              </li>
            ))}
          </ul>
          {approval.detail ? (
            <p className="border-t border-line px-2.5 py-1.5 font-medium text-fg-dim">
              {approval.detail}
            </p>
          ) : null}
        </div>
      ) : send && approval.detail ? (
        <p className="ml-11 rounded-[7px] border border-line bg-raised px-2.5 py-1.5 text-meta text-fg-dim">
          {approval.detail}
        </p>
      ) : null}
      <div className="flex justify-end gap-2">
        <Button
          variant="secondary"
          disabled={deciding}
          onClick={() => void decide(approval.id, false)}
        >
          Deny
        </Button>
        <Button
          variant="primary"
          icon="check"
          loading={deciding}
          disabled={deciding}
          onClick={() => void decide(approval.id, true)}
        >
          {send
            ? approval.items.length > 1
              ? `Approve and send ${approval.items.length}`
              : 'Approve and send'
            : 'Approve and ship'}
        </Button>
      </div>
    </section>
  );
}
