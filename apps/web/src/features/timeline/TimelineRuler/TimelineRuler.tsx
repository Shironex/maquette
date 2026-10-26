import { Icon, cn } from '@maquette/ui';
import { KEY, LANE, LANE_NAME, pct } from '../lanes';
import { useTimelineRuler } from './TimelineRuler.hooks';

/**
 * The page's scroll as a ruler, with the camera stops' names under it, and
 * the camera lane: a stop at each one's share of the page and the flight
 * between them. Drag the ruler to scrub the page.
 */
export default function TimelineRuler() {
  const r = useTimelineRuler();
  const ticks = r.ticks.map(t => (
    <span key={t.at}>
      <i
        className={cn('absolute top-0 w-px', t.major ? 'h-2 bg-fg-mute' : 'h-[5px] bg-line-strong')}
        style={{ left: pct(t.at) }}
      />
      {t.label ? (
        <span
          className="absolute top-[9px] -translate-x-1/2 whitespace-nowrap"
          style={{ left: pct(t.at) }}
        >
          {t.label}
        </span>
      ) : null}
    </span>
  ));
  const names = r.stops.map((stop, i) => (
    <span
      key={stop.id}
      title={stop.name}
      className={cn(
        'absolute top-6 max-w-[18%] truncate border-l border-line-strong pl-1.5 text-fg-dim',
        i === r.stops.length - 1 && r.stops.length > 1 && 'right-0 border-l-0 border-r pr-1.5 pl-0'
      )}
      style={i === r.stops.length - 1 && r.stops.length > 1 ? undefined : { left: pct(stop.at) }}
    >
      {stop.name}
    </span>
  ));
  const camera = r.stops.map((stop, i) => {
    const next = r.stops[i + 1];
    return (
      <span key={stop.id}>
        <button
          type="button"
          aria-label={`Camera stop ${i + 1}, ${stop.name}, at ${Math.round(stop.at)}%`}
          aria-current={i === r.current ? 'step' : undefined}
          onClick={() => r.jump(i)}
          className={cn(
            KEY,
            'bg-focus',
            i === r.current && 'shadow-[0_0_0_3px_var(--color-clay-soft)]'
          )}
          style={{ left: pct(stop.at) }}
        />
        {next ? (
          <span
            aria-hidden="true"
            className="absolute top-1.5 flex h-[22px] items-center overflow-hidden rounded-[5px] border border-dashed border-line-strong px-2 text-[11.5px] whitespace-nowrap text-fg-mute"
            style={{
              left: `calc(${pct(stop.at)} + 10px)`,
              width: `calc(${pct(next.at - stop.at)} - 20px)`,
            }}
          >
            Fly to {next.name}
          </span>
        ) : null}
      </span>
    );
  });
  return (
    <>
      <div className={cn(LANE_NAME, 'h-10 border-t-0')}>Page scroll</div>
      <div
        role="slider"
        tabIndex={0}
        aria-label="Playhead"
        aria-valuemin={0}
        aria-valuemax={100}
        aria-valuenow={Math.round(r.percent)}
        aria-valuetext={r.valueText}
        onPointerDown={r.grab}
        onPointerMove={r.scrub}
        onKeyDown={r.onKey}
        className="relative h-10 cursor-pointer border-l border-line font-mono text-[10.5px] leading-none text-fg-mute focus-visible:outline-2 focus-visible:outline-focus"
      >
        {ticks}
        {names}
      </div>
      <div className={LANE_NAME}>
        <Icon name="video-camera" size={14} className="text-fg-mute" />
        Camera
      </div>
      <div className={LANE}>{camera}</div>
    </>
  );
}
