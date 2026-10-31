import { useState } from 'react';

export type Layout = 'overlay' | 'side';

/** Whether the two rest poses are drawn over each other or side by side. */
export function useRestLayout() {
  const [layout, setLayout] = useState<Layout>('overlay');
  return { layout, setLayout };
}
