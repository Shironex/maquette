import { useState } from 'react';

/** Whether a stage's camera follows its model, on until turned off. */
export function useFollow() {
  const [follow, setFollow] = useState(true);
  return { follow, setFollow };
}
