import { BudgetBar, BudgetMeter, Tooltip } from '@maquette/ui';
import { firstViewLine, firstViewMb } from '@/state/layout';
import { usePipelineStore } from '@/state/pipeline-store';

/**
 * The scene budget for the top bar. Its tooltip says what the first camera
 * stop loads, the part of the scene a visitor waits for.
 */
export function SceneBudgetMeter({ value, target }: { value: number; target: number }) {
  const firstView = usePipelineStore(s => s.view?.firstView);
  return (
    <Tooltip
      label={`The whole scene: ${value.toFixed(1)} of ${target} MB. ${firstViewLine(firstView)}`}
      wrap
      side="bottom"
    >
      <span tabIndex={0} className="rounded-control">
        <BudgetMeter label="Scene budget" value={value} target={target} unit="MB" />
      </span>
    </Tooltip>
  );
}

/** What the first camera stop loads against its own budget, for the Pipeline view. */
export function FirstViewBar({ className }: { className?: string }) {
  const firstView = usePipelineStore(s => s.view?.firstView);
  if (!firstView) return null;
  const { value, target } = firstViewMb(firstView);
  return (
    <div className={className} title={firstViewLine(firstView)}>
      <BudgetBar
        label={firstView.stop ? `First view · ${firstView.stop}` : 'First view · no stop yet'}
        value={value}
        target={target}
        unit="MB"
        format={n => n.toFixed(1)}
        showNote={false}
      />
    </div>
  );
}
