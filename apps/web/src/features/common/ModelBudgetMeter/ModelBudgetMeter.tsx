import { Tooltip, cn } from '@maquette/ui';
import type { BudgetSummary } from '@/state/game';

export interface ModelBudgetMeterProps {
  summary: BudgetSummary;
}

/** The top bar's budget for a game: how many models are over their folder's budget. */
export default function ModelBudgetMeter({ summary }: ModelBudgetMeterProps) {
  const over = summary.over.length;
  const names = summary.over.map(a => a.name).join(', ');
  const label = over
    ? `Over their folder's budget: ${names}. Each model checks on its own; there is no scene total.`
    : 'Every model fits its folder budget. Each model checks on its own; there is no scene total.';
  const fill = summary.models ? ((summary.models - over) / summary.models) * 100 : 0;
  return (
    <Tooltip label={label} wrap side="bottom">
      <span tabIndex={0} className="rounded-control">
        <div className="flex min-w-42 flex-col gap-[5px]">
          <div className="flex items-baseline justify-between gap-3 text-[12px] leading-[14px] text-fg-mute">
            <span>Model budget</span>
            <span className={cn('font-mono text-[12px]', over ? 'text-amber' : 'text-fg')}>
              {over} over{' '}
              <span className="text-fg-mute">
                / {summary.models} {summary.models === 1 ? 'model' : 'models'}
              </span>
            </span>
          </div>
          <div
            role="meter"
            aria-label={`Model budget, ${over} of ${summary.models} models over`}
            aria-valuenow={summary.models - over}
            aria-valuemin={0}
            aria-valuemax={summary.models}
            className="relative h-1 overflow-hidden rounded-pill bg-amber/60"
          >
            <i className="absolute inset-y-0 left-0 block bg-sage" style={{ width: `${fill}%` }} />
          </div>
        </div>
      </span>
    </Tooltip>
  );
}
