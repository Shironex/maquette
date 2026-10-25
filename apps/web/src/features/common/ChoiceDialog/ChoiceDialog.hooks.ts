import { useRef } from 'react';
import { useTopEscape } from '@/lib/useTopEscape';

/** The dialog's box, which Escape dismisses while it is the top one. */
export function useChoiceDialog(onDismiss: () => void) {
  const ref = useRef<HTMLDivElement>(null);
  useTopEscape(ref, onDismiss);
  return ref;
}
