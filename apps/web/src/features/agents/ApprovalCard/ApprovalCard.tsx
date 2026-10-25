import { Button, Icon, cn } from '@maquette/ui';
import type { Approval } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { approvalTitle } from '@/state/agents';
import { formatCredits } from '@/state/credits';
import { useApprovalCard } from './ApprovalCard.hooks';

export interface ApprovalCardProps {
  approval: Approval;
  floating?: boolean;
}

export default function ApprovalCard({ approval, floating }: ApprovalCardProps) {
  const { decide, deciding, site, now, cost } = useApprovalCard(approval);
  const titleId = `${approval.id}-title`;
  const send = approval.kind === 'send-to-3d';
  const items = approval.items.map(item => (
    <li
      key={item.job}
      className="flex items-baseline gap-2 border-t border-line px-2.5 py-1 first:border-t-0"
    >
      <span className="flex-none font-medium text-fg">{item.assetName}</span>
      <span className="min-w-0 flex-1 truncate text-fg-mute" title={item.detail}>
        {item.detail}
      </span>
      {item.estimate !== null && item.estimate !== undefined ? (
        <span className="flex-none text-fg-dim tabular-nums">{formatCredits(item.estimate)}</span>
      ) : null}
    </li>
  ));
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
              An agent asked over MCP. This spends {cost}; nothing is sent until you approve.
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
          <ul className="m-0 flex max-h-[180px] list-none flex-col overflow-y-auto p-0">{items}</ul>
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
