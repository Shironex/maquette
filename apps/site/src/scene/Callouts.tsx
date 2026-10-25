/**
 * Pins the hero's scene labels (plain DOM, `[data-callout]` in the stage) to
 * the objects they name while the hero frame shows; the page fades them out
 * as the tour leaves it.
 */
import { useCallouts } from './Callouts.hooks';

export interface CalloutsProps {
  progress: React.RefObject<number>;
}

export function Callouts({ progress }: CalloutsProps) {
  useCallouts(progress);
  return null;
}
