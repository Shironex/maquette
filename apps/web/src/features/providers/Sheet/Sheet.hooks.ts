import { useCallback, useRef, useState } from 'react';
import { useTopEscape } from '@/lib/useTopEscape';

/**
 * Closing the sheet: Escape while it is on top, the backdrop and the close
 * button all dismiss it, and with unsent changes they ask first.
 */
export function useSheetDismiss(dirty: boolean, onClose: () => void) {
  const sheet = useRef<HTMLDivElement>(null);
  const [asking, setAsking] = useState(false);
  const dismiss = useCallback(() => (dirty ? setAsking(true) : onClose()), [dirty, onClose]);
  const keepEditing = () => setAsking(false);

  useTopEscape(sheet, dismiss);

  return { sheet, asking, dismiss, keepEditing };
}
