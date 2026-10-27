import { Button, SegmentedControl, Toggle, type SegmentedOption } from '@maquette/ui';
import type { LayoutStop, PathTrack, ScenePath } from '@/ipc/client';
import { addEnergize, energizeLine, removeEnergize, type PathsDraft } from '@/state/paths';
import { SectionTitle } from '../../SectionTitle';
import { usePathEnergize } from './PathEnergize.hooks';

type Driver = 'stops' | 'scroll' | 'always';

const DRIVERS: SegmentedOption<Driver>[] = [
  { value: 'stops', label: 'Camera stops' },
  { value: 'scroll', label: 'Scroll range', hint: 'Not built yet', disabled: true },
  { value: 'always', label: 'Always', hint: 'Not built yet', disabled: true },
];

export interface PathEnergizeProps {
  path: ScenePath;
  track: PathTrack | undefined;
  stops: readonly LayoutStop[];
  change: (fn: (draft: PathsDraft, id: string) => PathsDraft) => void;
}

/**
 * Whether the site lights the path as the page scrolls, and from where:
 * the amount lit at camera stops, keyed in the Timeline view.
 */
export default function PathEnergize({ path, track, stops, change }: PathEnergizeProps) {
  const { openTimeline } = usePathEnergize();
  return (
    <div className="flex flex-col gap-2.5">
      <SectionTitle>Energize</SectionTitle>
      <Toggle
        label="Light it as the page scrolls"
        description="A glow fills the path from its first point to its last."
        checked={track !== undefined}
        disabled={!track && stops.length === 0}
        onChange={on => change((d, id) => (on ? addEnergize(d, id, stops) : removeEnergize(d, id)))}
      />
      {!track && stops.length === 0 ? (
        <p className="text-meta text-fg-mute">
          Add a camera stop in the Layout tab first: the fill is keyed at stops.
        </p>
      ) : null}
      {track ? (
        <>
          <div className="flex min-h-7 items-center justify-between gap-2">
            <span className="text-fg-dim">Driven by</span>
            <SegmentedControl label="Driven by" options={DRIVERS} value="stops" />
          </div>
          <p className="text-meta text-fg-dim">{energizeLine(track, stops)}</p>
          <p className="text-meta text-fg-mute">
            Speed and direction are not built yet: the fill runs from the first point to the last,
            paced by the stops.
          </p>
          <Button
            size="sm"
            icon="film-strip"
            className="self-start"
            aria-label={`Edit ${path.name} in the Timeline`}
            tooltip="Opens the Timeline to set how much of this path is lit at each stop"
            onClick={openTimeline}
          >
            Edit in Timeline
          </Button>
        </>
      ) : null}
    </div>
  );
}
