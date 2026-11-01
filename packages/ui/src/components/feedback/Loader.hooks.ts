import { useEffect, useState } from 'react';

/**
 * False on the first render: a screen reader follows a live region that is
 * already there, so the words go into it just after it mounts.
 */
export function useMounted(): boolean {
  const [mounted, setMounted] = useState(false);
  useEffect(() => setMounted(true), []);
  return mounted;
}
