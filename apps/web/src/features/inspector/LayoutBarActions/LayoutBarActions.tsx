import { BudgetMeter, Button, Tooltip } from '@maquette/ui';
import { firstViewMb, firstViewLine } from '@/state/layout';
import { useLayoutBarActions } from './LayoutBarActions.hooks';
import { KEYS } from '@/lib/keys';

/** The first view meter, Revert and Save, for the top bar. */
export default function LayoutBarActions() {
  const { view, saving, save, revert, dirty } = useLayoutBarActions();
  const fv = firstViewMb(view?.firstView);
  return (
    <>
      <Tooltip label={firstViewLine(view?.firstView)} wrap side="bottom">
        <span tabIndex={0} className="rounded-control">
          <BudgetMeter label="First view" value={fv.value} target={fv.target} unit="MB" />
        </span>
      </Tooltip>
      <Button
        icon="arrow-u-up-left"
        disabled={!dirty || saving}
        disabledReason="No unsaved changes"
        tooltip="Drops your unsaved edits and goes back to the saved layout"
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
        tooltip="Saves layout.json; the next pipeline run builds the scene and its posters from it"
        shortcut={KEYS.save}
      >
        {saving ? 'Saving...' : 'Save layout'}
      </Button>
    </>
  );
}
