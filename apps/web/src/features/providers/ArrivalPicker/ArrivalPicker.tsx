/**
 * The downloads the watcher would not place on its own: the ones whose
 * name is only close to a waiting send's ("Needs a look") and the ones it
 * could not place at all ("Which asset is this?"). Each is attached to an
 * asset (sent to the Inbox for it, never moved) or left alone for good.
 */
import { useArrivalPickerOpen } from './ArrivalPicker.hooks';
import { AssetChooser } from './AssetChooser';
import { PickerDialog } from './PickerDialog';

export default function ArrivalPicker() {
  const { open, chooserFor, close } = useArrivalPickerOpen();
  if (!open) return null;
  return (
    <>
      <PickerDialog onClose={close} />
      {chooserFor ? <AssetChooser arrival={chooserFor} /> : null}
    </>
  );
}
