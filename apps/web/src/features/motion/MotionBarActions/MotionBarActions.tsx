import { BudgetMeter, Button, Tooltip } from '@maquette/ui';
import { ModelBudgetMeter } from '@/features/common/ModelBudgetMeter';
import { KEYS } from '@/lib/keys';
import { useMotionBarActions } from './MotionBarActions.hooks';

/**
 * The scene budget, Revert and Save motion, for the top bar. Motion has no
 * budget of its own: clips are bytes inside the models, so the meter is the
 * scene's, with the draft's clip choices in it. A game has no scene total
 * and nothing here to save: its bar shows the models against their folder
 * budgets, as the other views do.
 */
export default function MotionBarActions() {
  const { view, line, tip, saving, save, revert, dirty, models } = useMotionBarActions();
  if (models) return <ModelBudgetMeter summary={models} />;
  return (
    <>
      <Tooltip label={tip} wrap side="bottom">
        <span tabIndex={0} className="rounded-control">
          <BudgetMeter
            label="Scene budget"
            value={line?.sceneMb ?? 0}
            target={line?.targetMb ?? 3}
            unit="MB"
            format={mb => mb.toFixed(2)}
          />
        </span>
      </Tooltip>
      <Button
        icon="arrow-u-up-left"
        disabled={!dirty || saving}
        disabledReason="No unsaved changes"
        onClick={revert}
      >
        Revert
      </Button>
      <Button
        variant={dirty ? 'primary' : 'secondary'}
        icon="check"
        loading={saving}
        disabled={!view || !dirty}
        disabledReason="No unsaved changes"
        onClick={() => void save()}
        tooltip="Saves the states and clip choices into layout.json; the next run builds the scene with them"
        shortcut={KEYS.save}
      >
        {saving ? 'Saving…' : 'Save motion'}
      </Button>
    </>
  );
}
