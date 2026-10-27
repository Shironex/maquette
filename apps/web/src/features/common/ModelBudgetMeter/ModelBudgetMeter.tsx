import { Tooltip, cn } from '@maquette/ui';
import { formatK } from '@/lib/format';
import type { BudgetSummary } from '@/state/game';

export interface ModelBudgetMeterProps {
  summary: BudgetSummary;
}

/**
 * The top bar's figure for a game: how many models are over their folder's
 * budget (triangles or texture size, set per folder in Inspector > Budget).
 * The Inbox checks a model sent for an asset against the same budget.
 */
export default function ModelBudgetMeter({ summary }: ModelBudgetMeterProps) {
  const over = summary.over.length;
  const names = summary.over.map(a => a.name).join(', ');
  const folders = summary.folders
    .map(f => `${f.folder || 'top level'} ${formatK(f.triangles)} triangles, ${f.texture} px`)
    .join('; ');
  const limits = `The game's folder budgets: ${folders}. The Inbox checks a model sent for an asset against its folder's budget too. Add the fix-ups (Decimate, Shrink textures) in Inspector > Budget to bring models under.`;
  const label = over
    ? `Over the game budget: ${over} of ${summary.models} models, ${summary.overTriangles} on triangles and ${summary.overTextures} on texture size (${names}). ${limits}`
    : `Every model fits the game budget. ${limits}`;
  const fill = summary.models ? ((summary.models - over) / summary.models) * 100 : 0;
  return (
    <Tooltip label={label} wrap side="bottom">
      <span tabIndex={0} className="rounded-control">
        <div className="flex min-w-42 flex-col gap-[5px]">
          <div className="flex items-baseline justify-between gap-3 text-[12px] leading-[14px] text-fg-mute">
            <span>Game budget</span>
            <span className={cn('font-mono text-[12px]', over ? 'text-amber' : 'text-fg')}>
              {over} over{' '}
              <span className="text-fg-mute">
                / {summary.models} {summary.models === 1 ? 'model' : 'models'}
              </span>
            </span>
          </div>
          <div
            role="meter"
            aria-label={`Game budget, ${over} of ${summary.models} models over`}
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
