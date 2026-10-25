/**
 * Pins the hero's scene labels (plain DOM, `[data-callout]` in the stage) to
 * the objects they name. Only the hero frame shows them; the page's CSS
 * hides them on the other stops.
 */
import type { StopId } from './manifest';
import { useCallouts } from './Callouts.hooks';

export function Callouts({ stop }: { stop: React.RefObject<StopId> }) {
  useCallouts(stop);
  return null;
}
