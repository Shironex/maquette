import { useState } from 'react';
import { stignore } from '@/state/sync';

/** The recommended `.stignore` for this layout, and Copy with its "Copied" state. */
export function useSyncHelp(cache: string) {
  const [copied, setCopied] = useState(false);
  const text = stignore(cache);
  const copy = () => {
    void navigator.clipboard
      ?.writeText(text)
      .then(() => setCopied(true))
      .catch(() => undefined);
  };
  return { text, copied, copy };
}
