import { Icon, Tooltip, cn } from '@maquette/ui';
import type { PageLaneId } from '@/state/timeline-page';
import { BLOCK, KEY, LANE, LANE_NAME, pct } from '../lanes';
import { usePageLane } from './PageLane.hooks';

export interface PageLaneProps {
  lane: PageLaneId;
}

/**
 * A track for the whole page, the render style or the hero text: a dot at
 * each keyed stop, a block where the page fades between them, and a plus at
 * every other stop to key it there. Nothing without a track.
 */
export default function PageLane({ lane: id }: PageLaneProps) {
  const lane = usePageLane(id);
  if (!lane) return null;
  const spans = lane.spans.map(s => (
    <button
      key={`${s.from}-${s.to}`}
      type="button"
      tabIndex={-1}
      aria-hidden="true"
      onClick={() => lane.select(s.stop)}
      className={cn(BLOCK, 'border-line-strong bg-raised font-sans')}
      style={{ left: `calc(${pct(s.from)} + 8px)`, width: `calc(${pct(s.to - s.from)} - 16px)` }}
    >
      <Icon name={lane.icon} size={12} className="flex-none" />
      {s.label}
    </button>
  ));
  const keys = lane.keys.map(k => (
    <Tooltip key={k.stop} label={k.label}>
      <button
        type="button"
        aria-pressed={k.selected}
        aria-label={k.label}
        onClick={() => lane.select(k.stop)}
        className={cn(
          KEY,
          'rotate-0 rounded-full',
          k.selected ? 'bg-clay-text shadow-[0_0_0_3px_var(--color-clay-soft)]' : 'bg-fg-dim'
        )}
        style={{ left: pct(k.at) }}
      />
    </Tooltip>
  ));
  const adds = lane.free.map(stop => (
    <Tooltip key={stop.id} label={`Key ${lane.name} at ${stop.name}`}>
      <button
        type="button"
        aria-label={`Key ${lane.name} at ${stop.name}`}
        onClick={() => lane.add(stop.id)}
        className="absolute top-[9px] grid size-4 -translate-x-1/2 cursor-pointer place-items-center rounded-full border border-dashed border-line-strong bg-transparent p-0 text-fg-mute hover:border-fg-dim hover:text-fg"
        style={{ left: pct(stop.at) }}
      >
        <Icon name="plus" size={10} />
      </button>
    </Tooltip>
  ));
  return (
    <>
      <div className={cn(LANE_NAME, lane.active && 'bg-clay-soft text-fg')} title={lane.name}>
        <Icon name={lane.icon} size={14} className="flex-none text-fg-mute" />
        <span className="truncate">{lane.name}</span>
      </div>
      <div className={LANE} role="group" aria-label={lane.name}>
        {spans}
        {adds}
        {keys}
      </div>
    </>
  );
}
