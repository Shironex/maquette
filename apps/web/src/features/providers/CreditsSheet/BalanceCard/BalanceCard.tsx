import { cn } from '@maquette/ui';
import type { Balance, ProviderId } from '@/ipc/client';
import { balanceView } from '@/state/credits';
import { SERVICE_LABEL } from '@/state/providers';

export interface BalanceCardProps {
  service: ProviderId;
  balance: Balance | undefined;
  checking: boolean;
}

/** One provider's balance: credits left, with what running tasks hold. */
export default function BalanceCard({ service, balance, checking }: BalanceCardProps) {
  const view = balanceView(balance, checking);
  return (
    <div
      role="group"
      aria-label={`${SERVICE_LABEL[service]} balance`}
      className="flex flex-col gap-0.5 rounded-card border border-line bg-raised px-3.5 py-3"
    >
      <span className="text-caption text-fg-mute uppercase">{SERVICE_LABEL[service]}</span>
      <span
        className={cn(
          'text-[24px] leading-8 font-semibold tabular-nums',
          view.tone === 'ok' ? 'text-fg' : 'text-fg-mute'
        )}
      >
        {view.value}
      </span>
      <span
        className={cn('truncate text-meta', view.tone === 'error' ? 'text-danger' : 'text-fg-mute')}
        title={view.note}
      >
        {view.note}
      </span>
    </div>
  );
}
