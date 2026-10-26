import { IconButton, cn } from '@maquette/ui';
import { useStopScrubber } from './StopScrubber.hooks';

const pct = (share: number) => `${(share * 100).toFixed(2)}%`;

/**
 * The page's scroll as a strip of camera stops: drag or step through them
 * and every frame's page scrolls along, through the page bridge.
 */
export default function StopScrubber() {
  const {
    stops,
    count,
    answered,
    at,
    position,
    line,
    playing,
    seek,
    jump,
    prev,
    next,
    togglePlay,
  } = useStopScrubber();
  const last = Math.max(1, count - 1);
  const current = Math.round(at);
  const segments = stops.map((stop, i) => (
    <div
      key={`${i}-${stop.name}`}
      className="absolute top-0 bottom-0 overflow-hidden border-l border-line px-2 py-[5px] text-[11px] whitespace-nowrap text-fg-mute first:border-l-0"
      style={{ left: pct(i / last), width: pct(i === count - 1 ? 0 : 1 / last) }}
    >
      {stop.name}
    </div>
  ));
  const marks = stops.map((stop, i) => {
    const on = i === current;
    return (
      <button
        key={`${i}-${stop.name}`}
        type="button"
        aria-label={`Jump to stop ${i + 1}, ${stop.name}`}
        aria-current={on ? 'step' : undefined}
        onClick={() => jump(i)}
        className={cn(
          'absolute top-[26px] flex cursor-pointer items-center gap-[5px] border-0 bg-transparent p-0 text-[11.5px] font-medium whitespace-nowrap',
          on ? 'text-fg' : 'text-fg-dim hover:text-fg',
          i === count - 1 && count > 1
            ? '-translate-x-full flex-row-reverse'
            : '-translate-x-[11px]'
        )}
        style={{ left: pct(i / last) }}
      >
        <span
          className={cn(
            'grid size-[22px] place-items-center rounded-full font-mono text-[10.5px] font-bold',
            on ? 'bg-focus text-app' : 'border border-line-strong bg-hover text-fg-dim'
          )}
        >
          {i + 1}
        </span>
        {stop.name}
      </button>
    );
  });
  return (
    <section
      aria-label="Scroll scrubber"
      className="flex-none rounded-card border border-line bg-panel px-3.5 pt-2.5 pb-3"
    >
      <div className="mb-2 flex items-center gap-2.5">
        <h2 className="m-0 text-caption font-medium tracking-[0.04em] text-fg-mute uppercase">
          Scroll
        </h2>
        <span className="min-w-0 truncate font-mono text-[12px] text-fg tabular-nums">
          {line.percent}
          {line.detail ? <span className="text-fg-mute"> · {line.detail}</span> : null}
        </span>
        <span className="flex-1" />
        {answered ? null : (
          <span className="truncate text-meta text-fg-mute">
            The page has not answered yet: these are the project&apos;s camera stops
          </span>
        )}
        <IconButton
          icon="caret-left"
          label="Previous stop"
          size="sm"
          variant="secondary"
          onClick={prev}
          disabled={count < 2}
        />
        <IconButton
          icon={playing ? 'pause' : 'play'}
          label={playing ? 'Pause' : 'Play the scroll at reading speed'}
          size="sm"
          variant="secondary"
          onClick={togglePlay}
          disabled={count < 2}
        />
        <IconButton
          icon="caret-right"
          label="Next stop"
          size="sm"
          variant="secondary"
          onClick={next}
          disabled={count < 2}
        />
      </div>
      <div className="relative h-14">
        <div className="absolute inset-x-0 top-0 h-6 overflow-hidden rounded-control border border-line bg-raised">
          {segments}
        </div>
        <div
          aria-hidden="true"
          className="pointer-events-none absolute -top-1 h-8 w-0.5 -translate-x-px bg-clay-text"
          style={{ left: pct(position) }}
        />
        <input
          type="range"
          min={0}
          max={last}
          step={0.01}
          value={at}
          disabled={count < 2}
          aria-label="Scroll position"
          aria-valuetext={`${line.percent}${line.detail ? `, ${line.detail}` : ''}`}
          onChange={e => seek(Number(e.target.value))}
          className="absolute inset-x-0 top-0 h-6 w-full cursor-pointer opacity-0 disabled:cursor-default"
        />
        {marks}
      </div>
    </section>
  );
}
