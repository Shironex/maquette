import { StatusChip } from '@maquette/ui';
import type { Spend } from '@/ipc/client';
import { formatAgo } from '@/lib/format';
import { formatCredits, outcomeChip } from '@/state/credits';
import { SERVICE_LABEL } from '@/state/providers';

export interface SpendRowProps {
  spend: Spend;
  now: number;
  /** The project's name, when the list shows every project. */
  project: string | null;
}

/** One paid send: when, which asset, its settings, how it ended and what it cost. */
export default function SpendRow({ spend, now, project }: SpendRowProps) {
  const chip = outcomeChip(spend);
  const sent = spend.sentAtMs ? new Date(spend.sentAtMs).toLocaleString('en-GB') : '';
  return (
    <li className="grid grid-cols-[92px_minmax(0,1fr)_minmax(0,1.2fr)_96px_84px] items-center gap-3 border-t border-line px-3 py-2 first:border-t-0">
      <span className="text-meta text-fg-mute tabular-nums" title={sent}>
        {formatAgo(spend.sentAtMs ?? now, now)}
      </span>
      <span className="flex min-w-0 flex-col">
        <span className="flex min-w-0 items-center gap-1.5">
          <span className="truncate font-medium">{spend.assetName}</span>
          {spend.redoOf ? (
            <span
              title={`Redo of ${spend.redoOf}`}
              className="flex-none rounded-control border border-line-strong bg-raised px-1.5 text-[11px] font-medium text-fg-dim"
            >
              Redo
            </span>
          ) : null}
        </span>
        <span className="truncate text-meta text-fg-mute">
          {project ? `${project} · ` : ''}
          {SERVICE_LABEL[spend.service]}
          {spend.viaAgent ? ' · by an agent' : ''}
        </span>
      </span>
      <span className="truncate text-meta text-fg-dim" title={spend.summary}>
        {spend.summary}
      </span>
      <span>
        <StatusChip status={chip.status} label={chip.label} />
      </span>
      <span className="flex items-baseline justify-end gap-1 tabular-nums">
        {spend.estimated ? (
          <span className="text-[11px] font-medium text-fg-mute" title="From the price list">
            est.
          </span>
        ) : null}
        <span className="font-medium">{formatCredits(spend.credits)}</span>
      </span>
    </li>
  );
}
