import { useState } from 'react';

/** Which direction's menu is open; one at a time. */
export function useEarlierDirections() {
  const [menu, setMenu] = useState<string | null>(null);
  return { menu, setMenu };
}
