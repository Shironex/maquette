import { Kbd } from '@maquette/ui';
import { ExplodeLane } from '../ExplodeLane';
import { PathLane } from '../PathLane';
import { StateLane } from '../StateLane';
import { TimelineBar } from '../TimelineBar';
import { TimelineRuler } from '../TimelineRuler';
import { NAMES } from '../lanes';
import { useTimelineTracks } from './TimelineTracks.hooks';

const HINTS: readonly [readonly string[], string][] = [
  [['Space'], 'play'],
  [['[', ']'], 'jump a stop'],
];

/**
 * The page from top to bottom with a lane per track: the camera, each
 * prop's states, each explode track and each path's energize track. The
 * playhead is where the page preview is scrolled to.
 */
export default function TimelineTracks() {
  const { states, explode, paths, playhead } = useTimelineTracks();
  const hints = HINTS.map(([keys, what]) => {
    const kbds = keys.map(k => <Kbd key={k} keys={k} />);
    return (
      <span key={what} className="inline-flex items-center gap-1.5">
        {kbds}
        {what}
      </span>
    );
  });
  const stateLanes = states.map(m => <StateLane key={m.asset} asset={m.asset} />);
  const explodeLanes = explode.map(m => <ExplodeLane key={m.asset} asset={m.asset} />);
  const pathLanes = paths.map(p => <PathLane key={p.id} path={p.id} />);
  return (
    <section
      aria-label="Scroll timeline"
      className="flex flex-none flex-col rounded-card border border-line bg-panel"
    >
      <TimelineBar />
      <div className="relative grid" style={{ gridTemplateColumns: `${NAMES}px 1fr` }}>
        <TimelineRuler />
        {stateLanes}
        {explodeLanes}
        {pathLanes}
        <div
          aria-hidden="true"
          className="pointer-events-none absolute top-0 bottom-0 -ml-px w-0.5 bg-clay-text"
          style={{ left: `calc(${NAMES}px + (100% - ${NAMES}px) * ${playhead.percent / 100})` }}
        />
      </div>
      <div className="flex flex-wrap items-center gap-3 border-t border-line px-3 py-2 text-[11.5px] text-fg-mute">
        {hints}
        <span className="ml-auto">
          {states.length === 0
            ? 'Props with states in the Motion view get a lane here'
            : 'Click a stop or a block to edit it, a plus to add a change there'}
        </span>
      </div>
    </section>
  );
}
