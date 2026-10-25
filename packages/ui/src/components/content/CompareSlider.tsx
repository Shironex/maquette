import type { PointerEvent } from 'react';
import { Icon } from '../../icons/Icon';
import { cn } from '../../lib/cn';
import { useCompareSlider } from './CompareSlider.hooks';

export interface CompareSliderProps {
  before: string;
  after: string;
  beforeLabel?: string;
  afterLabel?: string;
  initial?: number;
  className?: string;
}

export function CompareSlider({
  before,
  after,
  beforeLabel = 'Before',
  afterLabel = 'After',
  initial = 50,
  className,
}: CompareSliderProps) {
  const { pos, ref, move, key } = useCompareSlider(initial);
  const tag =
    'absolute top-3 rounded-pill bg-scrim px-2 py-0.5 font-mono text-[11px] leading-4 text-fg';
  return (
    <div
      ref={ref}
      onPointerDown={(e: PointerEvent<HTMLDivElement>) => {
        e.currentTarget.setPointerCapture(e.pointerId);
        move(e.clientX);
      }}
      onPointerMove={(e: PointerEvent<HTMLDivElement>) => {
        if (e.buttons) move(e.clientX);
      }}
      className={cn(
        'relative aspect-video w-full touch-none overflow-hidden rounded-window bg-app select-none',
        className
      )}
    >
      <img
        src={before}
        alt={beforeLabel}
        className="pointer-events-none absolute inset-0 size-full object-cover"
      />
      <img
        src={after}
        alt={afterLabel}
        className="pointer-events-none absolute inset-0 size-full object-cover"
        style={{ clipPath: `inset(0 0 0 ${pos}%)` }}
      />
      <span className="absolute inset-y-0 -ml-px w-0.5 bg-fg" style={{ left: `${pos}%` }} />
      <button
        type="button"
        role="slider"
        aria-label="Compare position"
        aria-valuenow={Math.round(pos)}
        aria-valuemin={0}
        aria-valuemax={100}
        onKeyDown={key}
        className="absolute top-1/2 -mt-4 -ml-4 flex size-8 cursor-ew-resize items-center justify-center rounded-full bg-fg text-app shadow-raised"
        style={{ left: `${pos}%` }}
      >
        <Icon name="arrows-left-right" />
      </button>
      <span className={cn(tag, 'left-3')}>{beforeLabel}</span>
      <span className={cn(tag, 'right-3')}>{afterLabel}</span>
    </div>
  );
}
