import { useState } from 'react';
import { readFlag, writeFlag } from './storage';

const KEY = (id: string) => `maquette.tip.${id}`;

/** Whether a tip was dismissed, remembered per tip id on this machine. */
export function useDismissedTip(id: string): [dismissed: boolean, dismiss: () => void] {
  const [dismissed, setDismissed] = useState(() => readFlag(KEY(id)));
  return [
    dismissed,
    () => {
      setDismissed(true);
      writeFlag(KEY(id), true);
    },
  ];
}
