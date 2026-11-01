import { BudgetMeter, Button, Tooltip } from '@maquette/ui';
import { firstViewMb, firstViewLine } from '@/state/layout';
import { useLayoutBarActions } from './LayoutBarActions.hooks';
import { KEYS } from '@/lib/keys';

/** The first view meter, Revert and Save, for the top bar. */
export default function LayoutBarActions() {
  const { view, loadFailed, saving, save, revert, dirty } = useLayoutBarActions();
  const fv = firstViewMb(view?.firstView);
  const nothing = view ? 'No unsaved changes' : 'The layout has not been read yet';
  return (
    <>
      <Tooltip label={firstViewLine(view?.firstView)} wrap side="bottom">
        <span tabIndex={0} className="rounded-control">
          <BudgetMeter
            label="First view"
            value={view?.firstView.stop ? fv.value : null}
            target={fv.target}
            unit="MB"
            pending={view ? 'No camera stop' : loadFailed ? 'Could not read' : undefined}
          />
        </span>
      </Tooltip>
      <Button
        icon="arrow-u-up-left"
        disabled={!dirty || saving}
        disabledReason={nothing}
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
        disabledReason={nothing}
        onClick={() => void save()}
        tooltip="Saves layout.json; the next pipeline run builds the scene and its posters from it"
        shortcut={KEYS.save}
      >
        {saving ? 'Saving...' : 'Save layout'}
      </Button>
    </>
  );
}
