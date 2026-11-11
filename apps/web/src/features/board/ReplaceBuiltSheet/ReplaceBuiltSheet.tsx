import { Button, InlineBanner, Loader, Select } from '@maquette/ui';
import { Sheet } from '@/features/common/Sheet';
import type { ReplaceTarget } from '@/state/built-replace';
import { useReplaceBuiltSheet } from './ReplaceBuiltSheet.hooks';
import { SwapReport } from './SwapReport';
import { UndoSummary } from './UndoSummary';

export interface ReplaceBuiltSheetProps {
  project: string;
  /** The pair to confirm, or the asset whose replace it undoes. */
  target: ReplaceTarget;
}

const PICK = { value: '', label: 'Pick one...' };

/**
 * Use in place of a model, and its undo. A built asset takes the place of
 * another model: the asset keeps its id, its place in the layout and its
 * name, and the built model becomes its next version. The sheet reads what
 * would change first and shows it; nothing changes until Replace. Opened
 * for an asset a built model replaced, it shows what Bring back the old
 * model changes instead.
 */
export default function ReplaceBuiltSheet({ project, target }: ReplaceBuiltSheetProps) {
  const v = useReplaceBuiltSheet(project, target);
  const { plan } = v;
  const ready = plan.kind === 'replace' || plan.kind === 'undo';
  const title = v.undo
    ? `Bring back the old model of ${v.assetName}`
    : v.built
      ? `Use ${v.builtName} in place of a model`
      : 'Replace with a built asset';
  const meta = v.undo
    ? 'The model it had before is current again'
    : 'The asset keeps its id, its place in the layout and its name';
  const off = v.busy
    ? 'Working on it'
    : plan.kind === 'refused'
      ? 'This is not possible as it stands'
      : ready
        ? null
        : v.undo
          ? 'Reading what comes back'
          : 'Pick the built asset and the model it replaces';

  return (
    <Sheet
      id="replace-built"
      icon={v.undo ? 'arrow-u-up-left' : 'arrows-left-right'}
      title={title}
      meta={meta}
      width={600}
      onClose={v.close}
      footer={
        <div className="flex flex-wrap items-center justify-end gap-2 border-t border-line px-5 py-3">
          <Button tooltip="Closes this and changes nothing" onClick={v.close}>
            Cancel
          </Button>
          <Button
            variant="primary"
            icon={v.undo ? 'arrow-u-up-left' : 'arrows-left-right'}
            loading={v.busy}
            disabled={off !== null}
            disabledReason={off ?? undefined}
            tooltip={
              v.undo
                ? 'Makes the old model current again and takes the built versions off'
                : 'Puts the built model in its place. Bring back the old model undoes it'
            }
            onClick={() => void v.confirm()}
          >
            {v.undo ? 'Bring it back' : 'Replace'}
          </Button>
        </div>
      }
    >
      <div className="flex min-h-0 flex-1 flex-col gap-4 overflow-y-auto px-5 py-4">
        {v.undo ? null : (
          <div className="grid grid-cols-2 gap-3">
            <Select
              label="Built asset"
              hint="Its script and newest model take the place"
              options={[PICK, ...v.builtOptions]}
              value={v.built}
              onChange={v.setBuilt}
            />
            <Select
              label="Takes the place of"
              hint="Keeps its id, placement and name"
              options={[PICK, ...v.assetOptions]}
              value={v.asset}
              onChange={v.setAsset}
            />
          </div>
        )}
        {v.failed ? (
          <InlineBanner tone="danger" title="Nothing was changed" detail={v.failed} />
        ) : null}
        {plan.kind === 'refused' ? (
          <InlineBanner
            tone="warning"
            title={v.undo ? 'It cannot come back yet' : 'This pair cannot be swapped yet'}
            detail={plan.why}
          />
        ) : null}
        {plan.kind === 'reading' ? <Loader label="Reading what changes..." /> : null}
        {plan.kind === 'replace' ? <SwapReport report={plan.report} /> : null}
        {plan.kind === 'undo' ? <UndoSummary report={plan.report} /> : null}
        {plan.kind === 'idle' ? (
          <p className="text-[12.5px] text-fg-mute">
            Pick the built asset and the model it replaces. The sheet then shows both models side by
            side and what happens to every setting, before anything changes.
          </p>
        ) : null}
      </div>
    </Sheet>
  );
}
