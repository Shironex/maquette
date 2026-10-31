import { useState } from 'react';

/** Whether the log of a failed render is open. */
export function useLogToggle() {
  const [open, setOpen] = useState(false);
  return { open, toggle: () => setOpen(now => !now) };
}
