import { cn } from '@maquette/ui';
import type { McpCall } from '@/ipc/client';
import { clockTime } from '@/state/agents';
import { OUTCOME } from './outcome';

export interface ActivityRowProps {
  call: McpCall;
}

export default function ActivityRow({ call }: ActivityRowProps) {
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
