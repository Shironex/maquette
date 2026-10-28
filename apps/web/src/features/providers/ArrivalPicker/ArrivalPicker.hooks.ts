import { useDownloadsStore } from '@/state/use-downloads-store';

/** Whether the downloads picker is open, which download the asset chooser is for, and closing. */
export function useArrivalPickerOpen() {
  const open = useDownloadsStore(s => s.pickerOpen);
  const chooserFor = useDownloadsStore(s => s.chooserFor);
  const close = useDownloadsStore(s => s.closePicker);
  return { open, chooserFor, close };
}
