import { useState } from 'react';
import { useInboxStore } from '@/state/use-inbox-store';
import type { DiscardDialogProps } from './DiscardDialog.types';

/** The discard itself and whether it is running. */
export function useDiscardDialog({ item, onClose }: DiscardDialogProps) {
  const discard = useInboxStore(s => s.discard);
  const [busy, setBusy] = useState(false);

  const confirm = async () => {
    if (busy) return;
    setBusy(true);
    const done = await discard(item.id);
    // A failure is already on the notice; the dialog stays for another try.
    if (done) onClose();
    else setBusy(false);
  };

  return { busy, confirm };
}
