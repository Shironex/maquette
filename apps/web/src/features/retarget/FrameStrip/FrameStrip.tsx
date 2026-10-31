import type { KeyboardEvent, PointerEvent } from 'react';
import { cn } from '@maquette/ui';

export interface FrameStripProps {
  /** The clip's length; the strip runs from frame 0 to here. */
  frames: number;
  label: string;
  /** The trim, with handles to drag, when `onTrim` is given. */
  range?: readonly [number, number];
  onTrim?: (range: [number, number]) => void;
  /** Frames the review flagged. */
  flagged?: readonly [number, number] | null;
  flaggedLabel?: string;
  /** The playhead, and scrubbing it. */
  frame?: number;
  onSeek?: (frame: number) => void;
}

const at = (frame: number, frames: number) => `${frames > 0 ? (frame / frames) * 100 : 0}%`;

/** About six round tick marks along the strip, always with the last frame. */
function tickFrames(frames: number): number[] {
  const step = Math.max(1, Math.ceil(frames / 5 / 2) * 2);
  const out: number[] = [];
  for (let f = 0; f < frames - step / 2; f += step) out.push(f);
  return [...out, frames];
}

function frameUnder(e: PointerEvent<HTMLElement>, lane: HTMLElement | null, frames: number) {
  const box = lane?.getBoundingClientRect();
  const share = box && box.width > 0 ? (e.clientX - box.left) / box.width : 0;
  return Math.round(Math.min(Math.max(share, 0), 1) * frames);
}

/**
 * A clip's frames along a strip: the trim as two handles, a flagged range
 * and a playhead, each only when asked for. The handles and the playhead
 * take the arrow keys, a frame at a time.
 */
export default function FrameStrip(props: FrameStripProps) {
  const { frames, range, flagged, frame, onTrim, onSeek } = props;
  const [from, to] = range ?? [0, frames];
  const setEnd = (end: 'in' | 'out', f: number) => {
    if (!onTrim) return;
    onTrim(
      end === 'in'
        ? [Math.max(0, Math.min(f, to - 1)), to]
        : [from, Math.min(frames, Math.max(f, from + 1))]
    );
  };
  const arrows = (value: number, set: (f: number) => void) => (e: KeyboardEvent<HTMLElement>) => {
    const by = e.key === 'ArrowLeft' ? -1 : e.key === 'ArrowRight' ? 1 : 0;
    if (!by) return;
    e.preventDefault();
    set(value + by);
  };
  const handles = (['in', 'out'] as const).map(end => {
    const value = end === 'in' ? from : to;
    return (
      <span
        key={end}
        role="slider"
        tabIndex={0}
        aria-label={end === 'in' ? 'Trim in' : 'Trim out'}
        aria-valuemin={0}
        aria-valuemax={frames}
        aria-valuenow={value}
        onKeyDown={arrows(value, f => setEnd(end, f))}
        onPointerDown={e => {
          e.stopPropagation();
          e.currentTarget.setPointerCapture?.(e.pointerId);
        }}
        onPointerMove={e => {
          if (e.buttons === 1) {
            setEnd(end, frameUnder(e, e.currentTarget.parentElement, frames));
          }
        }}
        className="absolute top-1 h-7 w-2 -translate-x-1/2 cursor-ew-resize rounded-[3px] bg-fg focus-visible:outline-2 focus-visible:outline-clay"
        style={{ left: at(value, frames) }}
      />
    );
  });
  const ticks = tickFrames(frames).map(t => (
    <span
      key={t}
      className="absolute top-0 -translate-x-1/2 whitespace-nowrap"
      style={{ left: at(t, frames) }}
    >
      {t}
    </span>
  ));
  const scrub = (e: PointerEvent<HTMLDivElement>) => {
    if (onSeek && e.buttons === 1) onSeek(frameUnder(e, e.currentTarget, frames));
  };
  return (
    <div className="flex flex-col gap-1">
      <div
        role={onSeek ? 'slider' : undefined}
        tabIndex={onSeek ? 0 : undefined}
        aria-label={onSeek ? props.label : undefined}
        aria-valuemin={onSeek ? 0 : undefined}
        aria-valuemax={onSeek ? frames : undefined}
        aria-valuenow={onSeek ? frame : undefined}
        onKeyDown={onSeek ? arrows(frame ?? 0, onSeek) : undefined}
        onPointerDown={e => {
          if (!onSeek) return;
          e.currentTarget.setPointerCapture?.(e.pointerId);
          onSeek(frameUnder(e, e.currentTarget, frames));
        }}
        onPointerMove={scrub}
        className={cn(
          'relative h-9 rounded-control border border-line bg-raised',
          onSeek && 'cursor-pointer'
        )}
      >
        {range ? (
          <div
            aria-hidden="true"
            className="absolute inset-y-1 rounded-[4px] border border-clay-line bg-clay/20"
            style={{ left: at(from, frames), width: at(to - from, frames) }}
          />
        ) : null}
        {flagged ? (
          <div
            title={props.flaggedLabel}
            className="absolute inset-y-1 rounded-[4px] border border-amber/70 bg-amber-soft"
            style={{ left: at(flagged[0], frames), width: at(flagged[1] - flagged[0], frames) }}
          >
            <span className="sr-only">{props.flaggedLabel}</span>
          </div>
        ) : null}
        {onTrim ? handles : null}
        {frame !== undefined ? (
          <i
            aria-hidden="true"
            className="pointer-events-none absolute inset-y-0 w-0.5 bg-clay-text"
            style={{ left: at(frame, frames) }}
          />
        ) : null}
      </div>
      <div
        aria-hidden="true"
        className="relative mx-1 h-3.5 font-mono text-[10.5px] leading-none text-fg-mute"
      >
        {ticks}
      </div>
    </div>
  );
}
