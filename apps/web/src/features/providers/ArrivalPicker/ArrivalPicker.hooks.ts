import { useDownloadsStore } from '@/state/use-downloads-store';

/** Whether "Which asset is this?" is open, and how to close it. */
export function useArrivalPickerOpen() {
  const open = useDownloadsStore(s => s.pickerOpen);
  const close = useDownloadsStore(s => s.closePicker);
  return { open, close };
}
