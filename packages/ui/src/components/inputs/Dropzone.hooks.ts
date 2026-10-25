import { useState } from 'react';

/** True while files are dragged over the zone. */
export function useDragOver() {
  return useState(false);
}
