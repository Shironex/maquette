/** Dust drifting in the lamp's light (see ./Motes.hooks.ts). */
import { useMotes } from './Motes.hooks';

export interface MotesProps {
  /** Poster capture: the dust holds still. */
  still: boolean;
}

export function Motes({ still }: MotesProps) {
  const points = useMotes(still);
  return <primitive object={points} />;
}
