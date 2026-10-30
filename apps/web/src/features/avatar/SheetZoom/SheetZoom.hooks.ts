import { useState } from 'react';
import type { ZoomMode } from './SheetZoom';

/** Which sheet the zoom shows, starting where the person clicked. */
export function useSheetZoom(start: ZoomMode) {
  return useState<ZoomMode>(start);
}
