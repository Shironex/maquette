import { cn } from '@maquette/ui';
import type { GripSide } from '@/ipc/client';
import { handPicture, type HandPicture } from './hand-picture';

export interface GripPictureProps {
  /** Degrees per bone, X then Y then Z, as a grip holds them. */
  pose: Record<string, readonly (number | null)[]>;
  /** The hand that holds, drawn in clay; the other is drawn dim. */
  holds?: GripSide | null;
  label: string;
  className?: string;
}

function Hand({ hand, x, strong }: { hand: HandPicture; x: number; strong: boolean }) {
  const fingers = hand.fingers.map((points, i) => (
    <polyline
      key={i}
      points={points.map(([px, py]) => `${px},${py}`).join(' ')}
      fill="none"
      strokeWidth={i === 0 ? 5 : 4.5}
      strokeLinecap="round"
      strokeLinejoin="round"
      stroke="currentColor"
    />
  ));
  return (
    <g
      transform={`translate(${x} 0) rotate(${hand.tilt} 30 64)`}
      className={strong ? 'text-clay-text' : 'text-fg-mute'}
    >
      <rect x={21} y={60} width={18} height={18} rx={4} fill="currentColor" opacity={0.35} />
      <rect
        x={13}
        y={36}
        width={34}
        height={28}
        rx={8}
        fill="currentColor"
        opacity={0.22}
        stroke="currentColor"
        strokeWidth={1.5}
      />
      {fingers}
    </g>
  );
}

/**
 * A schematic hand drawn from a grip's pose: the palm, the thumb and four
 * fingers, each finger shorter the more its joints curl, the wrist tilted
 * as the hand bone turns. Both hands side by side when the pose turns
 * both, the holding hand in clay. Drawn, not rendered, so a grid of them
 * costs nothing.
 */
export default function GripPicture({ pose, holds, label, className }: GripPictureProps) {
  const sides = (['right', 'left'] as const).filter(side =>
    Object.keys(pose).some(bone => bone.startsWith(side))
  );
  const shown = sides.length ? sides : (['right'] as const);
  const hands = shown.map((side, i) => (
    <Hand
      key={side}
      hand={handPicture(pose, side)}
      x={i * 60}
      strong={!holds || holds === side || shown.length === 1}
    />
  ));
  return (
    <svg
      role="img"
      aria-label={label}
      viewBox={`0 0 ${shown.length * 60} 80`}
      className={cn('block h-full w-auto', className)}
    >
      {hands}
    </svg>
  );
}
