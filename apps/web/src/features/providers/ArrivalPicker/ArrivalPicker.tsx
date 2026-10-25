/**
 * "Which asset is this?": downloads the watcher could not place on its
 * own. Pick the waiting send each one belongs to and it is copied into the
 * inbox (never moved), or leave it alone for good.
 */
import { useArrivalPickerOpen } from './ArrivalPicker.hooks';
import { PickerDialog } from './PickerDialog';

export default function ArrivalPicker() {
  const { open, close } = useArrivalPickerOpen();
  if (!open) return null;
  return <PickerDialog onClose={close} />;
}
