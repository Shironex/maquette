import { Icon } from '@maquette/ui';
import { useSplitDrag } from './SplitHandle.hooks';

export interface SplitHandleProps {
  split: number;
  onChange: (split: number) => void;
}

/** The divider between the two sides, dragged or moved with the arrow keys. */
export default function SplitHandle({ split, onChange }: SplitHandleProps) {
  const { drag, move } = useSplitDrag(onChange);
  const left = `${split * 100}%`;
  return (
    <div ref={drag} className="pointer-events-none absolute inset-0">
      <span className="absolute inset-y-0 -ml-px w-0.5 bg-fg" style={{ left }} />
      <button
        type="button"
        role="slider"
        aria-label="Compare position"
        aria-valuenow={Math.round(split * 100)}
        aria-valuemin={0}
        aria-valuemax={100}
        onPointerDown={e => {
          e.currentTarget.setPointerCapture(e.pointerId);
        }}
        onPointerMove={e => {
          if (e.buttons) move(e.clientX);
        }}
        onKeyDown={e => {
          if (e.key === 'ArrowLeft') onChange(Math.max(0.02, split - 0.05));
          if (e.key === 'ArrowRight') onChange(Math.min(0.98, split + 0.05));
        }}
        className="pointer-events-auto absolute top-1/2 -mt-[18px] -ml-[18px] grid size-9 cursor-ew-resize place-items-center rounded-full bg-fg text-app shadow-[0_4px_16px_rgba(10,9,8,0.45)]"
        style={{ left }}
      >
        <Icon name="arrows-left-right" />
      </button>
    </div>
  );
}
