import { ChoiceDialog } from '@/features/common/ChoiceDialog';
import { PROVIDER_LABEL } from '@/state/providers';
import { MAX_REMOVE, assetsLabel, stopLabel } from '@/state/trash';
import { useRemoveAssetsDialog } from './RemoveAssetsDialog.hooks';
import type { RemoveAssetsDialogProps } from './RemoveAssetsDialog.types';

/** Most names the dialog lists before it says how many more. */
const NAMES_SHOWN = 8;

/**
 * Asks before Remove from project: names the cards, says where their files
 * go and lists the sends it stops. Paid sends being made keep their cards.
 */
export default function RemoveAssetsDialog(props: RemoveAssetsDialogProps) {
  const { assets, onClose } = props;
  const { plan, removable, tooMany, busy, error, confirm } = useRemoveAssetsDialog(props);
  const one = assets.length === 1;
  const label = assetsLabel(assets);
  const nameOf = (id: string) => assets.find(a => a.id === id)?.name ?? id;
  const more = assets.length - NAMES_SHOWN;
  const names = assets.slice(0, NAMES_SHOWN).map(a => (
    <li key={a.id} className="truncate">
      {a.name}
    </li>
  ));
  const stops = plan.stops.map(j => (
    <li key={j.id}>
      {nameOf(j.asset)}: {stopLabel(j)}
    </li>
  ));
  const making = plan.running.map(
    j => `${PROVIDER_LABEL[j.provider]} is making ${nameOf(j.asset)} now`
  );
  const removeLabel = one
    ? 'Remove'
    : `Remove ${assetsLabel(removable.length === assets.length ? assets : removable)}`;

  return (
    <ChoiceDialog
      title={`Remove ${label} from the project?`}
      onDismiss={onClose}
      choices={[
        {
          label: removeLabel,
          variant: 'danger',
          loading: busy,
          disabled: removable.length === 0 || tooMany,
          disabledReason: tooMany
            ? `Remove at most ${MAX_REMOVE} assets at a time`
            : 'Wait for its send to finish, or cancel it in the 3D queue',
          onClick: () => void confirm(),
        },
        { label: 'Cancel', onClick: onClose },
      ]}
    >
      <p>
        {one ? 'It leaves' : 'They leave'} this project&apos;s Board. {one ? 'Its' : 'Their'} refs,
        models, thumbnails and settings go to the workspace trash, and Recently removed on the Board
        can restore {one ? 'it' : 'them'}.
      </p>
      {one ? null : (
        <ul aria-label="Assets to remove" className="mt-2 flex flex-col gap-0.5 text-fg">
          {names}
          {more > 0 ? <li className="text-fg-mute">and {more} more</li> : null}
        </ul>
      )}
      {stops.length > 0 ? (
        <div className="mt-3">
          <p className="font-semibold text-fg">
            {stops.length === 1 ? 'This send stops' : `These ${stops.length} sends stop`}
          </p>
          <ul aria-label="Sends that stop" className="mt-1 flex flex-col gap-0.5">
            {stops}
          </ul>
        </div>
      ) : null}
      {making.length > 0 ? (
        <p role="note" className="mt-3 text-amber">
          {making.join('. ')}. {making.length === 1 ? 'It stays' : 'They stay'} until the send is
          over, so no credits are lost.
        </p>
      ) : null}
      {tooMany ? (
        <p role="note" className="mt-3 text-amber">
          Remove at most {MAX_REMOVE} assets at a time.
        </p>
      ) : null}
      {error ? (
        <p role="alert" className="mt-3 text-danger">
          {error}
        </p>
      ) : null}
    </ChoiceDialog>
  );
}
