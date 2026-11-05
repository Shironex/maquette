import type { KeyboardEvent, PointerEvent } from 'react';
import { NumberField } from '@maquette/ui';
import { frameOf, shareOf } from '@/state/recipe-draft';

export interface RecipeRangeProps {
  /** The base clip's frame count. */
  frames: number;
  /** Shares of the clip, 0 to 1. */
  start: number;
  end: number;
  onChange: (start: number, end: number) => void;
}

const pct = (share: number) => `${Math.min(Math.max(share, 0), 1) * 100}%`;
const KEY_STEP: Record<string, number> = {
  ArrowLeft: -1,
  ArrowDown: -1,
  ArrowRight: 1,
  ArrowUp: 1,
};

/**
 * Where a change starts and ends on the clip: a bar over the whole clip with
 * the part it covers, handles to drag or move with the arrow keys a frame at
 * a time, and the frames typed. The end always stays after the start.
 */
export default function RecipeRange({ frames, start, end, onChange }: RecipeRangeProps) {
  const from = frameOf(start, frames);
  const to = frameOf(end, frames);
  const set = (which: 'start' | 'end', frame: number) => {
    if (which === 'start') onChange(shareOf(Math.min(Math.max(frame, 0), to - 1), frames), end);
    else onChange(start, shareOf(Math.max(Math.min(frame, frames), from + 1), frames));
  };
  const drag = (which: 'start' | 'end') => (e: PointerEvent<HTMLButtonElement>) => {
    const lane = e.currentTarget.parentElement;
    if (!lane || e.buttons !== 1) return;
    const box = lane.getBoundingClientRect();
    set(
      which,
      Math.round(Math.min(Math.max((e.clientX - box.left) / (box.width || 1), 0), 1) * frames)
    );
  };
  const key = (which: 'start' | 'end') => (e: KeyboardEvent<HTMLButtonElement>) => {
    const step = KEY_STEP[e.key];
    if (!step) return;
    e.preventDefault();
    set(which, (which === 'start' ? from : to) + step * (e.shiftKey ? 10 : 1));
  };
  const handles = (['start', 'end'] as const).map(which => (
    <button
      key={which}
      type="button"
      aria-label={which === 'start' ? `Starts at frame ${from}` : `Ends at frame ${to}`}
      title="Drag, or use the arrow keys (Shift for 10 frames)"
      onPointerDown={e => e.currentTarget.setPointerCapture?.(e.pointerId)}
      onPointerMove={drag(which)}
      onKeyDown={key(which)}
      className="absolute top-0 h-5 w-2 -translate-x-1/2 cursor-ew-resize rounded-[3px] bg-fg focus-visible:outline-2 focus-visible:outline-focus"
      style={{ left: pct(which === 'start' ? start : end) }}
    />
  ));
  return (
    <div className="flex flex-col gap-1.5">
      <div role="group" aria-label="Where it plays on the clip" className="relative mx-1 h-5">
        <span className="absolute inset-x-0 top-2 h-1 rounded-pill bg-line" />
        <span
          className="absolute top-1.5 h-2 rounded-pill bg-clay"
          style={{ left: pct(start), width: pct(end - start) }}
        />
        {handles}
      </div>
      <div className="flex gap-2">
        <NumberField
          label="Start frame"
          size="sm"
          min={0}
          max={Math.max(to - 1, 0)}
          value={from}
          onChange={f => set('start', Math.round(f))}
          fieldClassName="flex-1"
        />
        <NumberField
          label="End frame"
          size="sm"
          min={Math.min(from + 1, frames)}
          max={frames}
          value={to}
          onChange={f => set('end', Math.round(f))}
          fieldClassName="flex-1"
        />
      </div>
    </div>
  );
}
