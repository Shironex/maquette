import { BudgetMeter, Tooltip } from '@maquette/ui';
import { ModelBudgetMeter } from '../ModelBudgetMeter';
import { useModelBudget, useSceneBudgetLabel } from './SceneBudgetMeter.hooks';
import type { SceneBudgetMeterProps } from './SceneBudgetMeter.types';

/**
 * The scene budget for the top bar. Its tooltip says what the first camera
 * stop loads, the part of the scene a visitor waits for. A game has no
 * scene total: it shows the models over their folder's budget instead.
 */
export default function SceneBudgetMeter({ value, target, pending }: SceneBudgetMeterProps) {
  const label = useSceneBudgetLabel(value, target);
  const models = useModelBudget();
  if (models) return <ModelBudgetMeter summary={models} />;
  return (
    <Tooltip label={label} wrap side="bottom">
      <span tabIndex={0} className="rounded-control">
        <BudgetMeter
          label="Scene budget"
          shortLabel="Scene"
          value={value}
          target={target}
          unit="MB"
          pending={pending}
        />
      </span>
    </Tooltip>
  );
}
