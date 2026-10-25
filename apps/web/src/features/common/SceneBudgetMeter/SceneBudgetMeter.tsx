import { BudgetMeter, Tooltip } from '@maquette/ui';
import { useSceneBudgetLabel } from './SceneBudgetMeter.hooks';
import type { SceneBudgetMeterProps } from './SceneBudgetMeter.types';

/**
 * The scene budget for the top bar. Its tooltip says what the first camera
 * stop loads, the part of the scene a visitor waits for.
 */
export default function SceneBudgetMeter({ value, target }: SceneBudgetMeterProps) {
  const label = useSceneBudgetLabel(value, target);
  return (
    <Tooltip label={label} wrap side="bottom">
      <span tabIndex={0} className="rounded-control">
        <BudgetMeter label="Scene budget" value={value} target={target} unit="MB" />
      </span>
    </Tooltip>
  );
}
