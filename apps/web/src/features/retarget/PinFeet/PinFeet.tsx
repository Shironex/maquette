/**
 * Pin the feet for a clip the review flags for sliding: bakes it again
 * holding each foot where it went down, the leg solved back to it in
 * Blender, then shows the slide before and after. A pin that slid more
 * came off again in the core, and this says so.
 */
import { Button, cn } from '@maquette/ui';
import type { ClipReview } from '@/ipc/client';
import { slideText } from '@/state/retarget';
import { usePinFeet } from './PinFeet.hooks';

export default function PinFeet({ clip }: { clip: ClipReview }) {
  const pin = usePinFeet(clip);
  const before = pin.before;
  if (pin.worse !== null) {
    return (
      <div
        role="group"
        aria-label="Pin left off"
        className="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-[13px]"
      >
        <span>
          <span className="text-meta text-fg-mute">Slide pinned </span>
          <span className="font-semibold text-amber tabular-nums">{slideText(pin.worse)}</span>
        </span>
        <span>
          <span className="text-meta text-fg-mute">Slide without </span>
          <span className="font-semibold tabular-nums">{slideText(pin.after ?? 0)}</span>
        </span>
        <span className="text-meta text-fg-mute">
          The pin slid more, so this bake leaves it off
        </span>
      </div>
    );
  }
  if (before !== null) {
    return (
      <div
        role="group"
        aria-label="Feet pinned"
        className="flex flex-wrap items-baseline gap-x-4 gap-y-1 text-[13px]"
      >
        <span>
          <span className="text-meta text-fg-mute">Slide before </span>
          <span className="font-semibold tabular-nums">{slideText(before)}</span>
        </span>
        <span>
          <span className="text-meta text-fg-mute">Slide after </span>
          <span
            className={cn('font-semibold tabular-nums', pin.still ? 'text-amber' : 'text-sage')}
          >
            {slideText(pin.after ?? 0)}
          </span>
        </span>
        <span className="text-meta text-fg-mute">
          {pin.still ? 'Feet pinned, still to look at' : 'Feet pinned while planted'}
        </span>
      </div>
    );
  }
  if (!pin.offer) return null;
  return (
    <div className="flex flex-wrap items-center gap-2">
      <Button size="sm" icon="map-pin" loading={pin.busy} disabled={pin.busy} onClick={pin.pin}>
        Pin the feet and bake again
      </Button>
      <span className="text-meta text-fg-mute">
        Holds each foot where it goes down and bends the leg to it. Saved with the clip settings.
      </span>
    </div>
  );
}
