import { Icon, Tooltip, cn } from '@maquette/ui';
import { BLOCK, LANE, LANE_NAME, pct } from '../lanes';
import { useStateLane } from './StateLane.hooks';

export interface StateLaneProps {
  asset: string;
}

/**
 * A prop's states along the page: a pill where arriving at a camera stop
 * changes it, a block where a scroll range moves it, and a small plus at
 * every other stop to add a change there.
 */
export default function StateLane({ asset }: StateLaneProps) {
  const lane = useStateLane(asset);
  if (!lane) return null;
  const marks = lane.marks.map(m =>
    m.kind === 'stop' ? (
      <button
        key={m.index}
        type="button"
        aria-pressed={m.selected}
        aria-label={`${lane.name}: ${m.label} at ${Math.round(m.start)}%`}
        onClick={() => lane.select(m.index)}
        className={cn(
          'absolute top-[5px] inline-flex h-[22px] -translate-x-[9px] cursor-pointer items-center gap-1 rounded-full border px-[7px] pl-[5px] font-mono text-[11px] font-medium whitespace-nowrap',
          m.selected
            ? 'border-clay-line bg-clay/22 text-fg'
            : 'border-line-strong bg-raised text-fg-dim hover:text-fg'
        )}
        style={{ left: pct(m.start) }}
      >
        <Icon name="video-camera" size={11} className="text-amber" />
        {m.label}
      </button>
    ) : (
      <button
        key={m.index}
        type="button"
        aria-pressed={m.selected}
        aria-label={`${lane.name}: ${m.label} from ${Math.round(m.start)}% to ${Math.round(m.end)}%`}
        onClick={() => lane.select(m.index)}
        className={cn(
          BLOCK,
          'font-sans',
          m.selected ? 'border-clay-line bg-clay/22' : 'border-amber/50 bg-amber-soft'
        )}
        style={{ left: pct(m.start), width: `max(${pct(m.end - m.start)}, 14px)` }}
      >
        <Icon name="mouse-scroll" size={12} className="flex-none" />
        {m.label}
      </button>
    )
  );
  const adds = lane.canAdd
    ? lane.free.map(stop => (
        <Tooltip key={stop.id} label={`Change ${lane.name} at ${stop.name}`}>
          <button
            type="button"
            aria-label={`Change ${lane.name} at ${stop.name}`}
            onClick={() => lane.add(stop.id)}
            className="absolute top-[9px] grid size-4 -translate-x-1/2 cursor-pointer place-items-center rounded-full border border-dashed border-line-strong bg-transparent p-0 text-fg-mute hover:border-fg-dim hover:text-fg"
            style={{ left: pct(stop.at) }}
          >
            <Icon name="plus" size={10} />
          </button>
        </Tooltip>
      ))
    : null;
  return (
    <>
      <div className={cn(LANE_NAME, lane.active && 'bg-clay-soft text-fg')} title={lane.name}>
        <Icon name="package" size={14} className="flex-none text-fg-mute" />
        <span className="truncate">{lane.name}</span>
      </div>
      <div className={LANE} role="group" aria-label={`${lane.name} states`}>
        {adds}
        {marks}
      </div>
    </>
  );
}
