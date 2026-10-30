import { useState } from 'react';
import type { SheetKind } from './SpringSheets.types';

/** Which sheet the zoom shows, starting on the one the person opened. */
export function useSpringSheets(start: SheetKind) {
  const [kind, setKind] = useState<SheetKind>(start);
  return { kind, setKind };
}
